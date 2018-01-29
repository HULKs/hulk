#ifndef UNIVALUE2JSONSTRING_H
#define UNIVALUE2JSONSTRING_H

#include <Tools/Storage/UniValue/UniValue.h>

namespace Uni{
  namespace Converter{
    std::string toJsonString(const Value &value, bool pretty = true);
  }
}

#endif
