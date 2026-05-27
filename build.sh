#!/usr/bin/env sh
set -eu

CC=${CC:-i686-pc-msdosdjgpp-gcc}
AR=${AR:-ar}
ARCH=${ARCH:-i486}
elf2djgpp=elf2djgpp
target=release

if [ "${1-}" = "debug" ]; then
    target="debug"
fi

RUST_XFLAGS=""
if [ "$target" = "release" ]; then
    RUST_XFLAGS="--release"
fi

RUST_TARGET="$ARCH-unknown-none-gnu"
echo "Target: $RUST_TARGET"

cargo build $RUST_XFLAGS --target ./$RUST_TARGET.json

APPNAME="whackac"
LIBNAME="lib$APPNAME.a"

# Extract the object files from the ELF static library
mkdir -p dist/$target/djgpp-lib
cd dist/$target/djgpp-lib
rm -f *.o
${AR} x "../../../target/$ARCH-unknown-none-gnu/$target/$LIBNAME"

echo "Converting ELF objects to COFF-GO32..."
for f in *.o; do
    $elf2djgpp -q "$f" "$f.new"
    rm -f "$f"
    mv "$f.new" "$f"
done
# clean up the previous one
rm -f "../$LIBNAME"
${AR} cr "../$LIBNAME" *.o

echo "$LIBNAME built"

C_XFLAGS="-march=${ARCH}"

if [ "$target" = "release" ]; then
    echo "Building release executable..."
    C_XFLAGS="$C_XFLAGS -O2 -s"
else
    echo "Building debug executable..."
    C_XFLAGS="$C_XFLAGS -O1 -g"
fi

$CC $C_XFLAGS -o ../$APPNAME.exe "../$LIBNAME"
echo "dist/$target/$APPNAME.exe built"
