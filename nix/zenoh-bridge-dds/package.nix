{
  lib,
  rustPlatform,
  fetchFromGitHub,
  cmake,
}:
rustPlatform.buildRustPackage {
  pname = "zenoh-bridge-dds";
  version = "1.10.2-booster-interop";

  src = fetchFromGitHub {
    owner = "eclipse-zenoh";
    repo = "zenoh-plugin-dds";
    rev = "4d3f5a154bb23650fa9c7fcbaed460207db6a99a";
    hash = "sha256-awylJ8kxxkj3famRxEF4kz27YuShSPR4jvpKGgwX3KI=";
  };

  cargoLock.lockFile = ./Cargo.lock;

  patches = [ ./patches/interop.patch ];

  postPatch = ''
    # cargoSetupPostUnpackHook provides the writable vendor tree in cargoDepsCopy.
    patch --batch --forward --directory="$cargoDepsCopy/cyclors-0.4.0" --strip=1 --fuzz=0 \
      < ${./patches/cyclone-booster-spdp.patch}
  '';

  nativeBuildInputs = [ cmake rustPlatform.bindgenHook ];
  cargoBuildFlags = [ "--bin" "zenoh-bridge-dds" ];

  meta = {
    description = "Zenoh DDS bridge with Booster firmware 1.8 interoperability fixes";
    homepage = "https://github.com/eclipse-zenoh/zenoh-plugin-dds";
    license = with lib.licenses; [ epl20 asl20 ];
    platforms = lib.platforms.linux;
    mainProgram = "zenoh-bridge-dds";
  };
}
