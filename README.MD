# A simple application to pauses / resume a given process


## How to use:

1) Select a process, make sure that you selected the main process (not a random subprocess)
2) The ui will change into a simple pause / resume button, press it to pause and resume your selected process

To select another process, re-start the app !

## Safety

The application is not signed, so you might run into the "Windows Protected Your PC" message.
You'll need to press more info -> run anyway.

If you do not trust the releases, well you can see the whole code (It's ugly af tho c:), feel free to compile it yourself !

Tested on Windows 10 w/ rustup 1.26.0 & rustc 1.76.0

## How does it work

Just like your task manager, simple calls to `NtResumeProcess` and `NtSuspendProcess`
