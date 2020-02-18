#!/bin/sh -l

target=$1

# '/github/workspace' is mounted as a volume and has owner set to root
# set the owner to the 'build' user, so it can access package files
sudo chown -R build /github/workspace

cd "$target"

namcap PKGBUILD && makepkg --printsrcinfo > .SRCINFO