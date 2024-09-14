The NAO has four microphones, which are located in the head.
They support frequencies between 100 and 10,000 Hz.

For more details, have a look at the [documentation](http://doc.aldebaran.com/2-8/family/nao_technical/microphone_naov6.html) by Aldebaran.

The audio cycler contains only two nodes, the microphone recorder and the whistle detection.

## Microphone Recorder

Reads the audio samples from the `MicrophoneInterface` and stores them in the audio cycler [database](../../framework/databases_and_types.md).

## Whistle Detection

Detects the whistle.
Similar to regular soccer, the referee uses a whistle to signal the start and end of the game.
More details on that can be found in the official [SPL rules](https://spl.robocup.org/wp-content/uploads/SPL-Rules-master.pdf)

The whistle detection works (simplified) by comparing the average power of the audio samples withthin a certain frequency band by using the [FFT](https://en.wikipedia.org/wiki/Fast_Fourier_transform).
This approach is not very advanced but works well in practice.

!!! tip

    The [Nao Devils](https://naodevils.de/) have put a lot of research into this topic and published datasets and [papers](https://naodevils.de/publications.html) regarding whistle detection and whistle localization.
