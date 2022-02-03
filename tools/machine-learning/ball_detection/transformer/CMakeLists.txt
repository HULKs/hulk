find_package(cxxopts CONFIG REQUIRED)
find_path(BOOST_CRC_INCLUDE_DIR "boost/crc.hpp")

set(TARGET_NAME transformer)
add_executable(${TARGET_NAME}
  "${CMAKE_CURRENT_SOURCE_DIR}/Configuration.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Configuration.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/main.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Processor.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Processor.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/TFRecordOffset.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/TFRecordOffset.hpp"
)

target_include_directories(${TARGET_NAME} PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/"
  "${BOOST_CRC_INCLUDE_DIR}"
)

target_link_libraries(${TARGET_NAME} PRIVATE
  cxxopts::cxxopts
)

install(TARGETS ${TARGET_NAME})
