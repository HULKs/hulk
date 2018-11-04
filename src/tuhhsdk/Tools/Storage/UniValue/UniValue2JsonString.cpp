#include "UniValue2JsonString.h"
#include "UniValue2Json.hpp"

#include <Libs/json/json.h>
#include <mutex>

namespace Uni
{
  namespace Converter
  {
    std::string toJsonString(const Value& value, bool pretty)
    {
      /// mutex for the json writers
      static std::mutex writerMutex;
      static Json::StyledWriter styledWriter;
      static Json::FastWriter fastWriter;

      /// as our json writers are not thread safe, we need to ensure that they are not called by
      /// different threads at the same time.
      std::lock_guard<std::mutex> lg(writerMutex);
      if (pretty)
      {
        return styledWriter.write(toJson(value));
      }
      else
      {
        return fastWriter.write(toJson(value));
      }
    }
  } // namespace Converter
} // namespace Uni
