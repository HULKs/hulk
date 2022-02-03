target_sources(hulk PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/GCAugmenterInterface.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/GameController.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/GameController.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/GameControllerAugmenter.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/GameControllerAugmenter.hpp"
)

add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/augmenterModules/")
