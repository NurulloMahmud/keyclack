# Third-party notices

## `assets/packs/cherrymx-black/press.wav`

Extracted from the "CherryMX Black - ABS keycaps" sound pack bundled with
[Mechvibes](https://github.com/hainguyents13/mechvibes)
(<https://mechvibes.com/sound-packs/sound-pack-1200000000001/>).

Mechvibes ships this pack as one of its own first-party default packs in
its main source tree (`src/audio/cherrymx-black-abs/`), under the
project's repository-wide license below — it carries no separate,
more restrictive license of its own.

`press.wav` here is a single ~226ms segment (key code `"1"`, offset
2894ms) extracted from Mechvibes' `sound.ogg` sprite sheet and converted
to 16-bit PCM mono WAV. Mechvibes' own format maps a distinct segment to
nearly every key for natural variation; `keyclack` only supports one
sample per pack, so this is one representative click rather than the
full per-key set.

```
MIT License

Copyright (c) 2021 Hai Nguyen

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

All other sound packs in `assets/packs/` are procedurally synthesized for
this project and carry no third-party attribution requirements.
