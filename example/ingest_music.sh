#!/bin/bash
echo "using filebot to process "$1""
filebot -rename -r "$1" --output /media/music --format "{artist}/{album}/{pi.pad(2)}_{t.lower().space('_')} {[af, kbps]}" --file-filter f.audio 
