#! /usr/bin/env python3
import serial
from serial.tools import list_ports
import struct
import argparse


class WDCMon:
    SYNC = 0
    WRITEMEM = 2
    READMEM = 3
    REGISTERS = 4
    EXEC = 5

    REGISTERS_ADDR = 0x7e00

    BOARD_VID = 0x0403
    BOARD_PID = 0x6001

    def __init__(self, port):
        self._serial = serial.Serial(port, 57600, rtscts=True)

    @classmethod
    def list_ports(cls, all=False):
        ports = []
        for port in list_ports.comports():
            if all or (
                    port.vid == cls.BOARD_VID and port.pid == cls.BOARD_PID):
                ports.append(port)
        return ports

    def _send_command(self, data):
        self._serial.write(b'\x55\xaa')
        resp = self._serial.read(1)
        assert resp == b'\xcc'
        self._serial.write(data)

    def _read_data(self, count):
        return self._serial.read(count)

    def close(self):
        self._serial.close()

    def sync(self):
        self._send_command(bytes([self.SYNC]))
        resp = self._read_data(1)
        assert resp == b'\x00'

    def readmem(self, address, count):
        cmd = struct.pack(
            '<BHBH',
            self.READMEM,
            address & 0xFFFF,
            address >> 16,
            count)
        self._send_command(cmd)
        return self._read_data(count)

    def writemem(self, address, data):
        cmd = struct.pack(
            '<BHBH',
            self.WRITEMEM,
            address & 0xFFFF,
            address >> 16,
            len(data))
        self._send_command(cmd + data)

    def run_program(self, start_address):
        regs = struct.pack(
            '<HHHHBBBBBBBB',
            0, 0, 0,  # A, X, Y
            start_address,
            0,  # direct page register
            0,
            255,  # stack pointer
            0,
            0,
            1,  # FIXME: cpu mode 1 for 65C02
            0,
            0
        )
        self.writemem(self.REGISTERS_ADDR, regs)
        self._send_command(bytes([self.EXEC]))


def read(mon, args):
    data = mon.readmem(args.start_address, args.size)
    with open(args.file_name, 'wb') as f:
        f.write(data)


def program(mon, args):
    with open(args.file_name, 'rb') as f:
        data = f.read()
    mon.writemem(args.start_address, data)

    if args.verify:
        read_data = mon.readmem(args.start_address, len(data))
        assert read_data == data


def run(mon, args):
    mon.run_program(args.start_address)


def auto_int(x):
    return int(x, 0)


def main():
    parser = argparse.ArgumentParser(description='Communicate with W65C816SXB')

    subparsers = parser.add_subparsers(
        help='subcommands',
        required=True,
        dest='command')

    list_parser = subparsers.add_parser('list', help='list serial devices')
    list_parser.add_argument('--all', '-a',
                             help='list all ports',
                             action='store_true')

    read_parser = subparsers.add_parser(
        'read',
        help='read data from the board')
    read_parser.add_argument('start_address', type=auto_int, help='start address')
    read_parser.add_argument('size', type=auto_int, help='amount of data to read')
    read_parser.add_argument('file_name', help='file to store data to')
    read_parser.add_argument(
        '--port', '-p',
        metavar='PORT',
        help='serial port to use')

    program_parser = subparsers.add_parser('program', help='program the board')
    program_parser.add_argument('start_address', type=auto_int, help='start address')
    program_parser.add_argument('file_name', help='file to read data from')
    program_parser.add_argument(
        '--port', '-p',
        metavar='PORT',
        help='serial port to use')
    program_parser.add_argument(
        '--verify',
        action='store_true',
        help='verify data after programming')

    run_parser = subparsers.add_parser('run', help='run program on the board')
    run_parser.add_argument('start_address', type=auto_int, help='start address')
    run_parser.add_argument(
        '--port', '-p',
        metavar='PORT',
        help='serial port to use')

    args = parser.parse_args()

    if args.command == 'list':
        ports = WDCMon.list_ports(all=args.all)
        devices = [port.device for port in ports]
        for device in sorted(devices):
            print(device)
    else:
        if args.port is not None:
            mon = WDCMon(args.port)
        else:
            all_ports = WDCMon.list_ports()
            if len(all_ports) != 1:
                raise Exception('Specify a serial port!')
            mon = WDCMon(all_ports[0].device)

        mon.sync()
        mon.sync()
        mon.sync()

        if args.command == 'read':
            read(mon, args)
        elif args.command == 'program':
            program(mon, args)
        elif args.command == 'run':
            run(mon, args)

        mon.close()


if __name__ == '__main__':
    main()
