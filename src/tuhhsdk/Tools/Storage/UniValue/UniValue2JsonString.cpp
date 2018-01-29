#include "UniValue2JsonString.h"
#include "UniValue2Json.hpp"
#include <Libs/json/json.h>

namespace Uni{
  namespace Converter{
    std::string toJsonString(const Value &value, bool pretty){
      static Json::StyledWriter styled_writer;
      static Json::FastWriter   fast_writer;

      if (pretty) {
        return styled_writer.write(toJson(value));
      }
      else {
        return fast_writer.write(toJson(value));
      }
    }
  }
}
