# Android offline speech notices

Family Room uses Vosk Android 0.3.75 for optional, device-local transcription of video audio. The application downloads `vosk-model-small-en-us-0.15` only when the user requests it. The model stays in app-private storage and is not included in backups. Both the Vosk API and this English model are listed under Apache License 2.0 by their publisher. A copy of the license is in [VOSK_LICENSE.txt](VOSK_LICENSE.txt).

- Vosk API and Android artifact: https://github.com/alphacep/vosk-api
- Model and published license listing: https://alphacephei.com/vosk/models
- Model archive SHA-256: `30f26242c4eb449f948e42cb302dd7a686cb29a3423a8367f99ff41780942498`

The optional model is English only. Transcription accuracy varies by audio quality and speaker; the app presents matches as timestamped evidence, not guaranteed answers.
