vcpkg_fail_port_install(ON_TARGET "uwp")
vcpkg_from_git(
  OUT_SOURCE_PATH SOURCE_PATH
  URL https://github.com/PortAudio/portaudio.git
  REF 9413fa051f0f9b830c117cb3dc4f6d675e330a99
  PATCHES
    0001-Merge-cmake_rewrite-of-github.com-Be-ing-portaudio.g.patch
    0001-Add-portaudio-namespace.patch
    0001-Add-find_dependency-to-portaudioConfig.cmake.in.patch
)

# NOTE: the ASIO backend will be built automatically if the ASIO-SDK is provided
# in a sibling folder of the portaudio source in vcpkg/buildtrees/portaudio/src
vcpkg_configure_cmake(
  SOURCE_PATH ${SOURCE_PATH}
  PREFER_NINJA
  OPTIONS
    -DJACK=OFF
    -DWASAPI=ON
    -DWDMKS=ON
    -DWMME=ON
    -DDLL_LINK_WITH_STATIC_RUNTIME=OFF
  OPTIONS_DEBUG
    -DDEBUG_OUTPUT:BOOL=ON
)

vcpkg_install_cmake()
vcpkg_fixup_cmake_targets(CONFIG_PATH lib/cmake/${PORT})
vcpkg_copy_pdbs()

file(REMOVE_RECURSE ${CURRENT_PACKAGES_DIR}/debug/include)
file(REMOVE_RECURSE ${CURRENT_PACKAGES_DIR}/debug/share)

# Handle copyright
file(INSTALL ${SOURCE_PATH}/LICENSE.txt DESTINATION ${CURRENT_PACKAGES_DIR}/share/${PORT} RENAME copyright)
