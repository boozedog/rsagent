{
  lib,
  rustPlatform,
  pkg-config,
  systemd,
  stdenv,
}:

rustPlatform.buildRustPackage {
  pname = "rsagent";
  version = "0.1.0";

  src = ../.;

  cargoLock.lockFile = ../Cargo.lock;

  buildFeatures = lib.optionals stdenv.hostPlatform.isLinux [ "systemd" "docker" ];

  nativeBuildInputs = [ pkg-config ];
  buildInputs = lib.optionals stdenv.hostPlatform.isLinux [ systemd ];

  env = lib.optionalAttrs stdenv.hostPlatform.isLinux {
    PKG_CONFIG_PATH = "${systemd}/lib/pkgconfig";
  };

  meta = with lib; {
    description = "Config-defined server agent CLI powered by aisdk";
    license = licenses.mit;
    platforms = platforms.linux ++ platforms.darwin;
    mainProgram = "rsagent";
  };
}
