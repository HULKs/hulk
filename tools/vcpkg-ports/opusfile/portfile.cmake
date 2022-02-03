vcpkg_check_linkage(ONLY_STATIC_LIBRARY)

if(VCPKG_CMAKE_SYSTEM_NAME STREQUAL WindowsStore)
  message(FATAL_ERROR "UWP builds not supported")
endif()

vcpkg_download_distfile(ARCHIVE_FILE
  URLS "https://downloads.xiph.org/releases/opus/opusfile-0.12.tar.gz"
  FILENAME "opusfile-0.12.tar.gz"
  SHA512 e25e6968a3183ac0628ce1000840fd6f9f636e92ba984d6a72b76fb2a98ec632d2de4c66a8e4c05ef30655c2a4a13ab35f89606fa7d79a54cfa8506543ca57af
)

vcpkg_extract_source_archive_ex(
  OUT_SOURCE_PATH SOURCE_PATH
  ARCHIVE ${ARCHIVE_FILE}
  PATCHES
    0001-Add-CMake.patch
)

if("opusurl" IN_LIST FEATURES)
  set(BUILD_OPUSURL ON)
else()
  set(BUILD_OPUSURL OFF)
endif()

vcpkg_configure_cmake(SOURCE_PATH ${SOURCE_PATH}
  PREFER_NINJA
  OPTIONS
    -DBUILD_OPUSURL=${BUILD_OPUSURL}
    -DOP_DISABLE_DOCS=ON
  OPTIONS_DEBUG
    -DOPUSFILE_SKIP_HEADERS=ON
)

vcpkg_install_cmake()
vcpkg_copy_pdbs()

file(REMOVE_RECURSE "${CURRENT_PACKAGES_DIR}/debug/include")
file(RENAME "${CURRENT_PACKAGES_DIR}/debug/lib/opusfile/opusfileTargets-debug.cmake" "${CURRENT_PACKAGES_DIR}/lib/opusfile/opusfileTargets-debug.cmake")
file(REMOVE_RECURSE "${CURRENT_PACKAGES_DIR}/debug")
file(MAKE_DIRECTORY "${CURRENT_PACKAGES_DIR}/share")
file(RENAME "${CURRENT_PACKAGES_DIR}/lib/opusfile" "${CURRENT_PACKAGES_DIR}/share/opusfile")

#file(READ "${CURRENT_PACKAGES_DIR}/share/opusfile/opusfileTargets.cmake" OPUSFILE_TARGETS_CMAKE)
#string(REPLACE "get_filename_component(_IMPORT_PREFIX \"\${_IMPORT_PREFIX}\" PATH)" "#get_filename_component(_IMPORT_PREFIX \"\${_IMPORT_PREFIX}\" PATH)" OPUSFILE_TARGETS_CMAKE "${OPUSFILE_TARGETS_CMAKE}")
#file(WRITE ${CURRENT_PACKAGES_DIR}/share/opusfile/opusfileTargets.cmake "${OPUSFILE_TARGETS_CMAKE}")

file(INSTALL ${SOURCE_PATH}/COPYING DESTINATION ${CURRENT_PACKAGES_DIR}/share/opusfile)
file(RENAME ${CURRENT_PACKAGES_DIR}/share/opusfile/COPYING ${CURRENT_PACKAGES_DIR}/share/opusfile/copyright)
