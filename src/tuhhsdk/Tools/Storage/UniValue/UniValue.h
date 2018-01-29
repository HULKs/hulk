#pragma once

#include <array>
#include <boost/variant.hpp>
#include <cstdint>
#include <iostream>
#include <list>
#include <map>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

#include "Tools/Math/Eigen.hpp"

namespace Uni
{
  enum class ValueType : uint8_t
  {
    NIL,
    INT,
    REAL,
    BOOL,
    STRING,
    ARRAY,
    OBJECT
  };

  class To;

  class Value
  {
  public:
    typedef std::map<std::string, Value> valuesMap_t;
    typedef std::vector<Value> valuesList_t;

    Value(ValueType = ValueType::NIL);
    Value(int32_t);
    Value(double);
    Value(bool);
    explicit Value(const std::string&);
    explicit Value(const char*);
    Value(const To&);

    Value& operator[](const char*);
    Value& operator[](const std::string&);
    Value& operator[](valuesList_t::size_type);
    Value& at(valuesList_t::size_type);
    const Value& operator[](const char*) const;
    const Value& operator[](const std::string&) const;
    const Value& operator[](valuesList_t::size_type) const;
    const Value& at(valuesList_t::size_type) const;


    Value& operator=(const Value&);

    ValueType type() const;

    int32_t asInt() const;
    double asDouble() const;
    bool asBool() const;
    const std::string asString() const;

    // non const versions
    valuesMap_t::iterator objectBegin();
    valuesMap_t::iterator objectEnd();
    valuesList_t::iterator listBegin();
    valuesList_t::iterator listEnd();

    // const versions
    valuesMap_t::const_iterator objectBegin() const;
    valuesMap_t::const_iterator objectEnd() const;
    valuesList_t::const_iterator listBegin() const;
    valuesList_t::const_iterator listEnd() const;

    valuesList_t::size_type size() const;
    void reserve(valuesList_t::size_type size);
    bool hasProperty(const std::string&) const;

  private:
    typedef boost::variant<int32_t, double, bool, std::string, valuesMap_t, valuesList_t> value_t;
    value_t value_;
    ValueType type_;
  };
}

inline void operator>>(const Uni::Value& in, Uni::Value& out)
{
  out = in;
}

inline void operator<<(Uni::Value& out, const Uni::Value& in)
{
  out = in;
}

inline void operator>>(const Uni::Value& in, int& out)
{
  out = in.asInt();
}

inline void operator<<(Uni::Value& out, const int in)
{
  out = Uni::Value(in);
}

inline void operator>>(const Uni::Value& in, unsigned int& out)
{
  out = in.asInt();
}

inline void operator<<(Uni::Value& out, const unsigned int in)
{
  out = Uni::Value((int)in);
}

inline void operator>>(const Uni::Value& in, double& out)
{
  out = in.asDouble();
}

inline void operator<<(Uni::Value& out, const double in)
{
  out = Uni::Value(in);
}

inline void operator>>(const Uni::Value& in, float& out)
{
  out = static_cast<float>(in.asDouble());
}

inline void operator<<(Uni::Value& out, const float in)
{
  out = Uni::Value(in);
}

inline void operator>>(const Uni::Value& in, bool& out)
{
  out = in.asBool();
}

inline void operator<<(Uni::Value& out, const bool in)
{
  out = Uni::Value(in);
}

inline void operator>>(const Uni::Value& in, std::string& out)
{
  out = in.asString();
}

inline void operator<<(Uni::Value& out, const std::string& in)
{
  out = Uni::Value(in);
}

#include "EigenStreaming.hpp"
#include "UniValueStreaming.hpp"
