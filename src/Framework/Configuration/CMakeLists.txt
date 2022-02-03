target_sources(hulk PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/ConfigMessageHeader.h"
  "${CMAKE_CURRENT_SOURCE_DIR}/Configuration.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Configuration.h"
  "${CMAKE_CURRENT_SOURCE_DIR}/NetworkConfig.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/NetworkConfig.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/UnixSocketConfig.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/UnixSocketConfig.hpp"
)
