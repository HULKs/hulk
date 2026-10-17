{
  lib,
  rustPlatform,
  fetchFromGitHub,
  cmake,
}:
rustPlatform.buildRustPackage {
  pname = "zenoh-bridge-dds";
  version = "1.9.0-booster-interop";

  # The standalone executable is not published on crates.io.
  src = fetchFromGitHub {
    owner = "eclipse-zenoh";
    repo = "zenoh-plugin-dds";
    rev = "f7101c8afd81be94dc96759e70b910539179b1c8";
    hash = "sha256-MOZmyZGaMJieWISZnnSTU+3+ER4FcGhg3YCTlmuWmuE=";
  };

  cargoLock = {
    lockFile = ./Cargo.lock;
    outputHashes = {
      "cyclors-0.3.10" = "sha256-h6n0iQ60mOIRpzC7aCO4bQi+SvkJQaN7UtGE/ohetjI=";
      "zenoh-plugin-dds-1.9.0" = "sha256-f8PTybkmL2f3C/YgR7wNOoQVgmtX35gBtrak5KMGifY=";
    };
  };

  postPatch = ''
    # Select the registry plugin so both Cargo patches take effect.
    substituteInPlace Cargo.toml \
      --replace-fail 'members = ["zenoh-bridge-dds", "zenoh-plugin-dds"]' 'members = ["zenoh-bridge-dds"]' \
      --replace-fail 'version = "1.9.0", path = "zenoh-plugin-dds/"' 'version = "=1.9.0"'
    cat >> Cargo.toml <<'EOF'
    [patch.crates-io]
    cyclors = { git = "https://github.com/IntelligentRoboticsLab/cyclors.git", rev = "8f58b83ac0f1c601d9fe9694cfe665b439a24dc8" }
    zenoh-plugin-dds = { git = "https://github.com/IntelligentRoboticsLab/zenoh-plugin-dds.git", rev = "2691f027f1f35e6d720d57585238540b35943655" }
    EOF
    cp ${./Cargo.lock} Cargo.lock
  '';

  nativeBuildInputs = [ cmake rustPlatform.bindgenHook ];
  cargoBuildFlags = [ "--bin" "zenoh-bridge-dds" ];
  doCheck = false;

  meta = {
    description = "Zenoh DDS bridge with Booster firmware 1.8 interoperability fixes";
    homepage = "https://github.com/eclipse-zenoh/zenoh-plugin-dds";
    license = with lib.licenses; [ epl20 asl20 ];
    platforms = lib.platforms.linux;
    mainProgram = "zenoh-bridge-dds";
  };
}
