#pragma once

#include "UniValue.h"

namespace Uni
{
  class To
  {
  public:
    /**
     * @brief ~To virtual destructor for polymorphism
     */
    virtual ~To()
    {
    }
    /**
     * @brief toValue converts a data structure to a Uni::Value
     * @param the new Uni::Value
     */
    virtual void toValue(Value& value) const = 0;
  };
  class From
  {
  public:
    /**
     * @brief ~From virtual destructor for polymorphism
     */
    virtual ~From()
    {
    }
    /**
     * @brief fromValue converts a Uni::Value to a data structure
     * @param value the Uni::Value that is to be converted
     */
    virtual void fromValue(const Value& value) = 0;
  };
}

inline void operator<<(Uni::Value& out, const Uni::To& in)
{
  in.toValue(out);
}

inline void operator>>(const Uni::Value& in, Uni::From& out)
{
  out.fromValue(in);
}
