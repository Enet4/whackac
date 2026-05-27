#!/usr/bin/env sh
set -eu

APPNAME="whackac"

# only call build.sh if the file was not built yet
if [ ! -f ./dist/release/$APPNAME.exe ]; then
    ./build.sh release
fi
mkdir -p ./dist/js-dos
# add the exe, assets, dosbox.conf and CWSDPMI.EXE into new zip file
cd bundle
rm -f dos_$APPNAME.zip
cp ../dist/release/$APPNAME.exe ./
zip -q dos_$APPNAME.zip \
    CWSDPMI.EXE \
    $APPNAME.exe \
    .jsdos/dosbox.conf
rm -f $APPNAME.exe

# rename it as dos_createac.jsdos
cp dos_$APPNAME.zip ../dist/dos_$APPNAME.jsdos
rm dos_$APPNAME.zip
echo "Created bundle dist/dos_$APPNAME.jsdos"
