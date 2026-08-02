# Third-party notices

VOD Scout 0.3.0은 아래 구성 요소를 YouTube 다운로드와 로컬 분석용으로 함께 배포합니다.

## yt-dlp

- Version: `2026.07.04`, official Windows x64 executable
- Upstream: https://github.com/yt-dlp/yt-dlp
- License: Unlicense; 공식 실행 파일에 포함된 제3자 구성 요소는 해당 배포물의 고지를 따름
- License copy: `media-tools/licenses/yt-dlp-Unlicense.txt`

## Deno

- Version: `2.9.4`, Windows x64
- Upstream: https://github.com/denoland/deno
- License: MIT
- License copy: `media-tools/licenses/Deno-MIT.md`

Deno는 yt-dlp가 YouTube JavaScript challenge를 처리할 때만 제한된 런타임으로 실행됩니다.

## FFmpeg / ffprobe

- Build: `n8.1.2-34-g9b6c8969e0-20260801`, Windows x64 LGPL shared
- Binary source: BtbN FFmpeg-Builds, `ffmpeg-n8.1-latest-win64-lgpl-shared-8.1.zip`
- Upstream: https://ffmpeg.org/
- Build scripts and corresponding source instructions: https://github.com/BtbN/FFmpeg-Builds
- License copy: `media-tools/licenses/FFmpeg-LGPL-3.0.txt`

VOD Scout는 FFmpeg 공유 DLL을 수정하지 않고 별도 프로세스로 호출합니다.

## whisper.cpp

- Version: `v1.9.1`, Windows CPU x64
- Upstream: https://github.com/ggml-org/whisper.cpp
- License: MIT
- License copy: `media-tools/licenses/whisper.cpp-MIT.txt`

## OpenAI Whisper model

- Model: multilingual `ggml-base.bin`
- Converted model source: https://huggingface.co/ggerganov/whisper.cpp
- Original project: https://github.com/openai/whisper
- License: MIT
- License copy: `media-tools/licenses/OpenAI-Whisper-MIT.txt`

`src-tauri/resources/media-tools/manifest.json`에는 내려받은 배포 파일의 원본 URL과 SHA-256이 기록돼 있습니다.
