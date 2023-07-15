#!/bin/bash
echo "using filebot to process "$1""
filebot -rename -r "$1" --output /media/wd_red/ --format "{plex}" --db TheMovieDB --lang en --log-file /home/plex/logs/filebot.log