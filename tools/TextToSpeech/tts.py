#!/usr/bin/env python3
from googleapiclient.discovery import build
from pathlib import Path
import json, base64, argparse

api_key = ''
api_key_needed = True
p = Path('.api_key')
if (p.exists()):
    api_key_needed = False
    with p.open('r') as f:
        api_key = f.readline()[:-1]

parser = argparse.ArgumentParser(description='Generate speech from text with google apis')
parser.add_argument('-a', '--api_key', required=api_key_needed, help='Google api key')
parser.add_argument('-l', '--list-voices', action='store_true', help='List voices')
parser.add_argument('-v', '--voice_name', help='The full name of the voice')
parser.add_argument('-o', '--output', help='The name of the output file')
parser.add_argument('text', type=str, nargs='*', help='The text to synthesize')
args = parser.parse_args()

if not args.list_voices and (args.voice_name is None or len(args.text) == 0 or args.output is None):
    parser.error("--voice_name, --output and text is required to synthesize text.")
if args.api_key is not None:
    api_key = args.api_key

# Supported voices:
# https://cloud.google.com/text-to-speech/docs/voices

tts = build('texttospeech', 'v1', developerKey=api_key)

if (args.list_voices):
    voices = tts.voices().list().execute()
    for voice in voices["voices"]:
        print("{} {}".format(voice["name"], voice["ssmlGender"]))
else:
    body = {
            "input": {
                "text": " ".join(args.text)
                },
            "voice": {
                "languageCode": args.voice_name[:5],
                "name": args.voice_name
                },
            "audioConfig": {
                "audioEncoding": "OGG_OPUS"
                }
            }

    response = tts.text().synthesize(body=body).execute()
    ogg = base64.b64decode(response["audioContent"])

    with open(args.output, 'wb') as f:
        f.write(ogg)
