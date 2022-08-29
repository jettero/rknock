# knock knock

I don't like traditional port knocking... it's too easy for someone with tcpdump
to construct a replay. This is my attempt to improve on that slightly.

This project should in no way be considered to be more secure than the usual
port knocking though. We're relying on openssh (or whatever) to do the real
work.

This just opens the port for a few seconds.
