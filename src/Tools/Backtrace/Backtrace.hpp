#pragma once

#include <cxxabi.h>   // for __cxa_demangle
#include <dlfcn.h>    // for dladdr
#include <execinfo.h> // for backtrace

#include <cstdio>
#include <cstdlib>
#include <sstream>
#include <string>

/**
 * @brief This function produces a stack backtrace with demangled function & method names.
 *
 * From https://gist.github.com/fmela/591333
 *
 * @param skip the number of stack frames to skip (from start)
 * @return a string containing the stack
 */
inline std::string backtrace(int skip = 1)
{
  constexpr int nMaxFrames = 128;
  void* callstack[nMaxFrames];
  char buf[1024];
  int nFrames = backtrace(callstack, nMaxFrames);
  char** symbols = backtrace_symbols(callstack, nFrames);

  std::ostringstream trace_buf;
  for (int i = skip; i < nFrames; i++)
  {
    Dl_info info;
    if (dladdr(callstack[i], &info) && info.dli_sname)
    {
      char* demangled = nullptr;
      int status = -1;
      if (info.dli_sname[0] == '_')
      {
        demangled = abi::__cxa_demangle(info.dli_sname, nullptr, 0, &status);
      }
      /*
       * %-3d: Stack frame number
       * %*p : return address, requires two arguments: Width and address
       * %s  : demangled function name
       * %zx : offset into function
       */
      snprintf(buf, sizeof(buf), "%-3d %*p %s + 0x%zx\n", i, int(2 + sizeof(void*) * 2),
               callstack[i], status == 0 ? demangled : info.dli_sname,
               (char*)callstack[i] - (char*)info.dli_saddr);
      free(demangled);
    }
    else
    {
      /*
       * %-3d: Stack frame number
       * %*p : return address, requires two arguments: Width and address
       * %s  : function name as returned by backtrace_symbols()
       */
      snprintf(buf, sizeof(buf), "%-3d %*p %s\n", i, int(2 + sizeof(void*) * 2), callstack[i],
               symbols[i]);
    }
    trace_buf << buf;
  }
  free(symbols);
  if (nFrames == nMaxFrames)
    trace_buf << "[truncated]\n";
  return trace_buf.str();
}
