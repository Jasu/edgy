Edgy
====

Run commands or enable / disable touch screen by swiping over screen edges on 
multi-touch screens.

Compiling
---------

   cargo build

Running
-------

First, list your input devices by running **xinput**. Find the name of your 
touch screen.

Then (substituting your touch screen device for *ELAN Touchscreen*), run e.g.


    edgy -d "ELAN Touchsreen" -a "from left to right with two fingers run 'xeyes'" \ 
                              -a "down to up with one finger toggle touch screen"
    
Actions
-------

  * **run command 'command args'** Runs command args. Commands are run in the 
    background and are not terminated when **Edgy** terminates.
  * **toggle touchscreen** Toggles touch-input for other applications.
  * **enable touchscreen** Enables touch-input (if disabled) for other 
    applications.
  * **disable touchscreen** Disables touch-input (if disabled) for other 
    applications.


Note
----

The code is unnecessarily complex, since this was a learning project. Action 
parsing is implemented using [nom](https://github.com/Geal/nom) and various 
different forms ("run 'xeyes' from left to right with three fingers" vs. 
"right from left exec command 'xeyes'")

License
-------
Edgy is licensed under the ISC license.

    ISC License

    Copyright (c) 2016, Jasper Mattsson

    Permission to use, copy, modify, and/or distribute this software for any
    purpose with or without fee is hereby granted, provided that the above 
    copyright notice and this permission notice appear in all copies.

    THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
    WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF 
    MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR 
    ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
    WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN 
    ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF 
    OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
