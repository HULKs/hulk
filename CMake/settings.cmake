if(NOT (CMAKE_CXX_COMPILER_ID MATCHES "MSVC"))
  set(CMAKE_CONFIGURATION_TYPES Debug Develop Release CACHE STRING "" FORCE)

  if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Develop CACHE STRING "" FORCE)
  endif(NOT CMAKE_BUILD_TYPE)
endif(NOT (CMAKE_CXX_COMPILER_ID MATCHES "MSVC"))

set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Check for the GNU compiler.
if(CMAKE_CXX_COMPILER_ID MATCHES "GNU")
  message(STATUS "Compiling with GCC")

  # Check the version of the compiler.
  if(CCACHE AND NOT FORCE_NO_CCACHE)
    execute_process(COMMAND "${CMAKE_CXX_COMPILER_ARG1}" -dumpversion OUTPUT_VARIABLE GCC_VERSION)
  else(CCACHE AND NOT FORCE_NO_CCACHE)
    execute_process(COMMAND "${CMAKE_CXX_COMPILER}" -dumpversion OUTPUT_VARIABLE GCC_VERSION)
  endif(CCACHE AND NOT FORCE_NO_CCACHE)
  if(GCC_VERSION VERSION_LESS 5)
    message(FATAL_ERROR "Compiling the main part of the code needs C++14."
      "Your GCC seems to be too old to support this."
      "GCC 5 is the first version that fully supports C++14.")
  endif(GCC_VERSION VERSION_LESS 5)

  # Enable as many warnings as possible and handle them as errors unless a release is compiled.
  set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra -pedantic -pipe")
  set(CMAKE_CXX_FLAGS_DEBUG "-Werror -pedantic-errors -g -fno-omit-frame-pointer")
  set(CMAKE_CXX_FLAGS_DEVELOP "-Werror -pedantic-errors -O3 -fomit-frame-pointer")
  set(CMAKE_CXX_FLAGS_RELEASE "-O3 -DNDEBUG -fomit-frame-pointer")
elseif(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
  message(STATUS "Compiling with Clang")

  # Set the same options as with gcc.
  set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra -pedantic -pipe")
  set(CMAKE_CXX_FLAGS_DEBUG "-Werror -pedantic-errors -g -fno-omit-frame-pointer")
  set(CMAKE_CXX_FLAGS_DEVELOP "-Werror -pedantic-errors -O3 -fomit-frame-pointer")
  set(CMAKE_CXX_FLAGS_RELEASE "-O3 -DNDEBUG -fomit-frame-pointer")
elseif(CMAKE_CXX_COMPILER_ID MATCHES "MSVC")
  message(STATUS "Compiling with MSVC")

  if(MSVC_VERSION LESS 1910)
    message(FATAL_ERROR "Compiling the main part of the code needs C++14."
      "Your Microsoft Visual C++ version is too old to support this."
      "Microsoft Visual C++ 15.0 is the first version that fully supports C++14.")
  endif(MSVC_VERSION LESS 1910)

  # Enable all warnings and handle them as errors.
  set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} /W2 /wd4244 /wd4305 /MP")
  set(CMAKE_CXX_FLAGS_DEVELOP ${CMAKE_CXX_FLAGS_DEBUG})
  set(CMAKE_EXE_LINKER_FLAGS_DEVELOP ${CMAKE_EXE_LINKER_FLAGS_DEBUG})
  set(CMAKE_SHARED_LINKER_FLAGS_DEVELOP ${CMAKE_SHARED_LINKER_FLAGS_DEBUG})
  add_definitions(-D_SCL_SECURE_NO_WARNINGS -D_WIN32_WINNT=0x0501)
else(CMAKE_CXX_COMPILER_ID MATCHES "GNU")
  message(FATAL_ERROR "This code can only be compiled with gcc, clang and Microsoft Visual C++, but your compiler ID is ${CMAKE_CXX_COMPILER_ID}")
endif(CMAKE_CXX_COMPILER_ID MATCHES "GNU")

if(WIN32)
  add_definitions(-D_USE_MATH_DEFINES -DEIGEN_DONT_VECTORIZE -DEIGEN_DISABLE_UNALIGNED_ARRAY_ASSERT)
endif(WIN32)

if(NAO)
  add_definitions(-DEIGEN_DONT_VECTORIZE -DEIGEN_DISABLE_UNALIGNED_ARRAY_ASSERT)
endif(NAO)

# from http://stackoverflow.com/a/31423421 (corrected with an underscore)
function(assign_source_group)
    foreach(_source IN ITEMS ${ARGN})
        if (IS_ABSOLUTE "${_source}")
          file(RELATIVE_PATH _source_rel "${CMAKE_CURRENT_SOURCE_DIR}" "${_source}")
        else()
          set(_source_rel "${_source}")
        endif()
        get_filename_component(_source_path "${_source_rel}" PATH)
        string(REPLACE "/" "\\" _source_path_msvc "${_source_path}")
        source_group("${_source_path_msvc}" FILES "${_source}")
    endforeach()
endfunction(assign_source_group)
