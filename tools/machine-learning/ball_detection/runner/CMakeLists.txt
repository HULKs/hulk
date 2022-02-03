find_package(TBB CONFIG REQUIRED)
find_package(Threads REQUIRED)

set(TARGET_NAME runner)
add_library(${TARGET_NAME} INTERFACE)

target_include_directories(${TARGET_NAME} INTERFACE
  "${CMAKE_CURRENT_SOURCE_DIR}/"
)

target_link_libraries(${TARGET_NAME} INTERFACE
  TBB::tbb
  Threads::Threads
)
