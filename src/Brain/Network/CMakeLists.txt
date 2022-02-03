target_sources(hulk PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/NetworkService.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/NetworkService.hpp"
)

add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/SPLNetwork/")
