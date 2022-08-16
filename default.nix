(
  import
  (
    fetchTarball
    (with (builtins.fromJSON (builtins.readFile ./flake.lock)).nodes.flake-compat.locked; {
      url = "https://github.com/${owner}/${repo}/archive/${rev}.tar.gz";
      sha256 = narHash;
    })
  )
  {src = ./.;}
)
.defaultNix
