target_sources(hulk PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/main.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/NaoAudio.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/NaoAudio.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/NaoCamera.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/NaoCamera.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/NaoFakeData.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/NaoFakeData.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/NaoInterface.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/NaoInterface.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/ProxyInterface.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/V4L2CtrlSetting.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/V4L2CtrlSetting.hpp"
)
