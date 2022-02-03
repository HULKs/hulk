include(vcpkg_common_functions)

vcpkg_from_github(
  OUT_SOURCE_PATH SOURCE_PATH
  REPO banditcpp/bandit
  REF v2.0.0
  SHA512 0419633514c46cd0fb369a9b1a139a2f2a27e2afb1eadd807ad769726917fd16e38a5676d45ba78c9d2ecd923c21503d9b4f8cfc95186c4306613c8cf6df2dc8
  HEAD_REF master
)

file(COPY ${SOURCE_PATH}/bandit DESTINATION ${CURRENT_PACKAGES_DIR}/include/ FILES_MATCHING PATTERN *.h)
file(COPY ${SOURCE_PATH}/LICENSE.md DESTINATION ${CURRENT_PACKAGES_DIR}/share/bandit)
file(COPY ${CMAKE_CURRENT_LIST_DIR}/FindBandit.cmake DESTINATION ${CURRENT_PACKAGES_DIR}/share/bandit)
file(RENAME ${CURRENT_PACKAGES_DIR}/share/bandit/LICENSE.md ${CURRENT_PACKAGES_DIR}/share/bandit/copyright)

vcpkg_from_github(
  OUT_SOURCE_PATH SOURCE_PATH
  REPO banditcpp/snowhouse
  REF v3.1.0
  SHA512 ea5d6b4b4560752925807f9ece201960764563650473fd80159cfafc0b960c8a3a8a719e937886f3af53ed1ae3d0e4b016a1611700318afa58a2e3365562a7c4
  HEAD_REF master
)

file(COPY ${SOURCE_PATH}/include/snowhouse DESTINATION ${CURRENT_PACKAGES_DIR}/include/bandit/assertion_frameworks/snowhouse FILES_MATCHING PATTERN *.h)
