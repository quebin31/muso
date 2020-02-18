target=$1

# '/github/workspace' is mounted as a volume and has owner set to root
# set the owner to the 'build' user, so it can access package files
sudo chown -R build /github/workspace

cd "$target"

namcap PKGBUILD && makepkg --printsrcinfo > .SRCINFO
version=$(echo $GITHUB_REF | cut -d '/' -f3)

export GIT_SSH_COMMAND="ssh -i $HOME/.ssh/aur -o StrictHostKeyChecking=no"
git config --local user.email "action@github.com"
git config --local user.name "GitHub Action"
git commit -m "updver: $version" -a
git push origin master
unset GIT_SSH_COMMAND