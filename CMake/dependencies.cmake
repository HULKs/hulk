if(WIN32)
  find_package(Boost COMPONENTS system date_time filesystem REQUIRED)
else(WIN32)
  find_package(Boost COMPONENTS system filesystem REQUIRED)
endif(WIN32)

find_package(Threads REQUIRED)
find_package(JPEG REQUIRED)
find_package(ZLIB REQUIRED)
find_package(PNG REQUIRED)
find_package(FFTW REQUIRED)
find_package(ITTNotify)
find_package(OpenCV 4.0.1 REQUIRED)
# Eigen 3.3 is required because in earlier versions normalize(d) would be NaN when the vector to be normalized is 0.
find_package(Eigen3 3.3 REQUIRED)
# This is needed since FindEigen3.cmake does not call find_package_handle_standard_args most of the time.
if(NOT EIGEN3_FOUND)
  message(FATAL_ERROR "Your Eigen version is too old!")
endif(NOT EIGEN3_FOUND)

if(NAO_V6)
  find_package(msgpack REQUIRED)
endif(NAO_V6)

find_package(PORTAUDIO REQUIRED)
find_package(OPUSFILE REQUIRED)

set(TUHH_DEPS_INCLUDE_DIRECTORIES
  ${JPEG_INCLUDE_DIR}
  ${ZLIB_INCLUDE_DIRS}
  ${PNG_INCLUDE_DIRS}
  ${FFTW_INCLUDE_DIRS}
  ${EIGEN3_INCLUDE_DIR}
  ${OpenCV_INCLUDE_DIRS}
  ${PORTAUDIO_INCLUDE_DIRS}
  ${SNDFILE_INCLUDE_DIRS}
  ${OPUSFILE_INCLUDE_DIRS}
  ${MSGPACK_INCLUDE_DIR})

set(TUHH_DEPS_LIBRARIES
  Boost::system
  Boost::filesystem
  ${CMAKE_THREAD_LIBS_INIT}
  ${JPEG_LIBRARIES}
  ${ZLIB_LIBRARIES}
  ${PNG_LIBRARIES}
  ${FFTW_LIBRARIES}
  ${MSGPACK_LIBRARIES}
  ${OpenCV_LIBS}
  ${PORTAUDIO_LIBRARIES}
  ${SNDFILE_LIBRARIES}
  ${OPUSFILE_LIBRARIES}
  ${CMAKE_DL_LIBS})

if (${ITTNOTIFY_FOUND})
  add_definitions(-DITTNOTIFY_FOUND)
  set(TUHH_DEPS_INCLUDE_DIRECTORIES ${TUHH_DEPS_INCLUDE_DIRECTORIES} ${ITTNOTIFY_INCLUDE_DIRS})
  set(TUHH_DEPS_LIBRARIES ${TUHH_DEPS_LIBRARIES} ${ITTNOTIFY_LIBRARIES})
endif (${ITTNOTIFY_FOUND})

if(NAO)
  set(TUHH_DEPS_INCLUDE_DIRECTORIES
    ${TUHH_DEPS_INCLUDE_DIRECTORIES})

  set(TUHH_DEPS_LIBRARIES
    ${TUHH_DEPS_LIBRARIES}
    ${CMAKE_DL_LIBS}
    -lrt)
endif(NAO)

message(STATUS "Include directories: ${TUHH_DEPS_INCLUDE_DIRECTORIES}")
message(STATUS "Link libraries: ${TUHH_DEPS_LIBRARIES}")
