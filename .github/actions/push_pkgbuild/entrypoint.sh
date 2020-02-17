target=$1
sshkey=$2

# '/github/workspace' is mounted as a volume and has owner set to root
# set the owner to the 'build' user, so it can access package files
sudo chown -R build /github/workspace

cd "$target"

namcap PKGBUILD && makepkg --printsrcinfo > .SRCINFO

echo $sshkey > ssh.key
git config --local core.sshCommand "ssh -i $(pwd)/ssh.key -F /dev/null"
git config --local user.email "action@github.com"
git config --local user.name "GitHub Action"
git commit -m "Updated from actions" -a
git push origin master