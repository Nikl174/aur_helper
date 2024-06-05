{lib, rustPlatform, openssl, pkg-config, sudo}:
let 
    pname = "aur_helper";
    version = "0.1.1";
    in
rustPlatform.buildRustPackage {
  pname = pname;

  # TODO: get from cargo.toml
  version = version;

  src =  builtins.fetchTarball {
    url ="https://github.com/Nikl174/${pname}/archive/refs/tags/${version}.tar.gz";
    sha256 = "15ac9pqw1hww8g8mik12x5bhz2v3j8d8vdbpmq3jd7arrq35jj2a";
  };

  nativeBuildInputs = [pkg-config];

  builtInputs = [openssl openssl.dev pkg-config sudo];

  cargoHash ="sha256-MrhxWNbot5rGI6EC1Uyzjl3xa67xSXAHPUDDuLVdRF4=";

  PKG_CONFIG_PATH = "${openssl.dev}/lib/pkgconfig";

  meta = {
    description = "a simple aur helper programm for managing aur packages in one directory";
    homepage = "https://github.com/Nikl174/${pname}";
    license = lib.licenses.unlicense;
    maintainers = ["nikl174@mailbox.org"];
  };

  postInstall = ''
    install -Dm0755 -t "$out/share/zsh/site-functions/" "completion/_${pname}"
  '';

}
