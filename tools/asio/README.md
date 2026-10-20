# ASIO test dependencies

The three headers in this directory are Steinberg ASIO SDK headers, retrieved
from the `audiosdk/asio` mirror on 2026-10-18. This is a mirror rather than an
official Steinberg repository. Preserve their copyright notices and
`LICENSE.txt`. This test host selects the GPLv3 licensing option. The headers
are only used by the test tool, not by the proposed application audio path.

Source files:

- https://github.com/audiosdk/asio/blob/main/common/asio.h
- https://github.com/audiosdk/asio/blob/main/common/asiosys.h
- https://github.com/audiosdk/asio/blob/main/common/iasiodrv.h
- https://github.com/audiosdk/asio/blob/main/LICENSE.txt

Steinberg's official announcement independently confirms GPLv3 availability:
https://ocl-steinberg-live.steinberg.net/_storage/asset/808575/storage/master/Press%20Release%20-%202025-10-15%20-%20OBS%20Partnership-%20EN.pdf

Recorded SHA-256:

| File | SHA-256 |
|---|---|
| asio.h | 6d83d9b6815d21c002b055c628860e8b0a1ca94c5ae1d9d18f137e308ba98096 |
| asiosys.h | 387fc21d036c3f39d7f74704bebb1e9cfe0bd8564ccd48088827162b06afe583 |
| iasiodrv.h | 0b7d9786fc77ad47dc16324629eb2ae9bff716a0edb023727df3c7ab1b59bf6d |
| LICENSE.txt | 61e56814f24b32216884b2db0b4dcc317111a5ab354d54d19dd9fc13d71b1c7a |
