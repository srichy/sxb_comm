# sxb_comm: interface for the WDC W65C816SXB

This is a terminal-based program which allows the user to:
1. hex-dump regions of memory
2. binary-dump regions of memory
3. upload binaries to specific memory locations
4. direct the wdcmon to execute code at a specified location
on the Western Design Center W65C816SXB single-board computer.  It
will _probably_ also work for the W65C02SXB board, but I can't test
that since I do not have one.

This code is super-dumb-simple and both tries to set the baud rate to
a reasonably low value _and_ adds explicit time delays between bytes.
This was to avoid overruns of the USB serial-to-VIA chip to CPU data
path.  The calls to `write_all` using single-byte slices of even very
short buffers was a hack-around that I just left as-is.  Sorry for the
ugly code.

Credit to Chris Baird's `sxb.py` that I found in a forum somewhere.  I
can't find it on github or I would link directly.  He figured out the
wire protocol, especially for the execute data block.  That saved me
lots of time in Ghidra trying to reverse it.  The documentation for
wdcmon only gives reasonable detail for things like reading and
writing memory blocks, and that's pretty trivial.

# On clap usage

Yes, I need to re-org the cli and use more of clap's power.  I went
quick-and-dirty with the somewhat ad hoc argument, the format of which
is predicated on the "mandatory option" (irony intended) which
implies how to interpret the format of the argument.  Ugly but
functional, at least for now.
