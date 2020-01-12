# nightwriter

*Jot down notes in the dark with a simple and invisible append-only
text editor*

Note: Only supports Linux + X11. Also, just wrote this, not very
well tested yet.

The motivation for this program is to be able to confidently type
without looking at the screen. The specific usecase is having a
keyboard next to the bed at night, and be able to type notes, ideas,
dreams, without looking at a screen. This way you can take notes in
total darkness.

## How it works

When you run `nightwriter`, it fully grabs your keyboard, which means
no other programs will receive your keystrokes. You can then type one
of the following:

* **`ctrl+shift+escape`, the only way to exit.**

* Typing digits, letters, and symbols, appends to the end of the
  output file.

* **`backspace`**, deletes the last character.

* **`ctrl+backspace`**, deletes the last word. A word is anything that
  isn't whitespace. If the last character was whitespace, deletes all
  of the trailing whitespace, and then deletes the last word.

* All other keystrokes get ignored.

If you invoke `nightwriter notes-file`, it will create or append to
`notes-file`.  If you don't specify an output file,

Nightwriter currently has no display. This may change in the future,
but for now a decent way to simulate a display is to run:

```
$ touch notes-file
$ tail -f notes-file
```

And then, in another terminal in the same directory:

```
$ nightwriter notes-file
```

If you omit the file name it will default to `night-YYYY-MM-DD`.

## Is this serious?  Why not use pen and paper and a dim light?

Hmmm, good question!

## Other uses

While night time writing is my primary usecase, I can imagine some
other uses as well.

* Tool for visually impaired people.  I am unfamiliar with the needs
  and software tools of visually impaired people, but something like
  this might be helpful!

* Overcoming writer's block and distractions.  Being unable to see
  what you've written is a fun restriction to play with.  It really
  hampers the editorial influence of our inner perfectionist, which
  can be great for getting thoughts down rapidly.

* Creating a spectacle at the local cafe by typing into a keyboard
  without a computer in sight.

* Recording thoughts while witnessing a beautiful sunset, without the
  nuisance of an interposing screen.

* Watching serene nature videos while typing, without the distraction
  of seeing the words you have typed.

* Staring into the starry night, recording galactic thoughts by poking
  at keys.

* Sitting around a campfire, philosophizing into your bluetooth
  keyboard, while getting mesmerized by the flames. Someone told me
  that they sometimes did this. I don't remember who, unfortunately,
  but talking with them definitely planted the seed of the idea for
  nightwriter.

## Potential future features

I'm not sure how much more I will add to nightwriter.  Here are some
feature ideas:

* Customizable keybindings which run some external program and pass
  some subset of the input to nightwriter.  Lots of potential uses for
  this:

  - Invoke a text-to-speech synthesizer on a chunk of recent input.

  - Send the most recently typed chunk of input to myself via email.

  - Add the most recently typed chunk of text to my todo list.

  - Play a sound to reassure yourself that nightwriter is functioning.

  - A special command to send `TERM` signal to all invoked processes.
    Primary use of this is to kill in progress text-to-speec
    synthesizers.

  - Probably special commands to switch to different files.

* Daemon mode - a mode which binds just one shortcut key which
  initiates nightwriter.  Exiting with **`ctrl+shift+escape`** would
  just re-enter the listening daemon mode.

* Add output to stdout like `nightwriter -`, this would disable
  special handling of backspace.  Probably not worth the added
  implementation complexity.

* It would be cool to support things other than Linux + X11. However,
  doing this would practically be a whole new program, as most of the
  code has to do with grabbing the X11 keyboard and processing X11 key
  events. Since I wrote this for fun and my own use, supporting other
  environments is not my priority.

## Contributing

Please do open PRs and issues on GitHub!
