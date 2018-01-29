#include "UniValue.h"
#include "UniConvertible.hpp"
#include <math.h>

namespace Uni
{
  Value::Value(int32_t i)
    : value_(i)
    , type_(ValueType::INT)
  {
  }
  Value::Value(double d)
    : value_(d)
    , type_(ValueType::REAL)
  {
  }
  Value::Value(bool b)
    : value_(b)
    , type_(ValueType::BOOL)
  {
  }
  Value::Value(const std::string& s)
    : value_(s)
    , type_(ValueType::STRING)
  {
  }
  Value::Value(const char* s)
    : value_(std::string(s))
    , type_(ValueType::STRING)
  {
  }
  Value::Value(const To& t)
  {
    t.toValue(*this);
  }

  Value::Value(ValueType t)
    : type_(t)
  {
    switch (t)
    {
      case ValueType::NIL:
        break;
      case ValueType::INT:
        value_ = (int)0;
        break;
      case ValueType::REAL:
        value_ = (double)0.0;
        break;
      case ValueType::BOOL:
        value_ = false;
        break;
      case ValueType::STRING:
        value_ = std::string();
        break;
      case ValueType::ARRAY:
        value_ = valuesList_t();
        break;
      case ValueType::OBJECT:
        value_ = valuesMap_t();
        break;
    }
  }

  Value& Value::operator[](const char* key)
  {
    if (type_ == ValueType::NIL)
    {
      type_ = ValueType::OBJECT;
      value_ = valuesMap_t();
    }

    if (type_ != ValueType::OBJECT)
    {
      throw std::runtime_error("Can not apply eckige klammer string to non object type!");
    }
    valuesMap_t& map = boost::get<valuesMap_t>(value_);
    return map[key];
  }

  const Value& Value::operator[](const char* key) const
  {
    if (type_ != ValueType::OBJECT)
    {
      throw std::runtime_error("Can not apply eckige klammer string to non object type!");
    }
    const valuesMap_t& map = boost::get<valuesMap_t>(value_);
    return map.at(key);
  }

  Value& Value::operator[](const std::string& key)
  {
    return (*this)[key.c_str()];
  }

  const Value& Value::operator[](const std::string& key) const
  {
    return (*this)[key.c_str()];
  }

  Value& Value::operator[](Value::valuesList_t::size_type pos)
  {
    if (type_ == ValueType::NIL)
    {
      type_ = ValueType::ARRAY;
      value_ = valuesList_t();
    }

    if (type_ != ValueType::ARRAY)
      throw std::runtime_error("Uni::Value::operator[] is only useful for arrays (value was not)");
    valuesList_t& lst = boost::get<valuesList_t>(value_);
    if (pos >= lst.size())
      lst.resize(pos + 1);
    return boost::get<valuesList_t>(value_)[pos];
  }

  const Value& Value::operator[](Value::valuesList_t::size_type pos) const
  {
    if (type_ != ValueType::ARRAY)
      throw std::runtime_error("Uni::Value::operator[] (const)"
                               "is only useful for arrays (value was not)");
    return boost::get<valuesList_t>(value_)[pos];
  }

  Value& Value::at(Value::valuesList_t::size_type pos)
  {
    return (*this)[pos];
  }

  const Value& Value::at(Value::valuesList_t::size_type pos) const
  {
    return (*this)[pos];
  }

  Value& Value::operator=(const Value& rhs)
  {
    value_ = rhs.value_;
    type_ = rhs.type_;
    return *this;
  }

  ValueType Value::type() const
  {
    return type_;
  }

  int32_t Value::asInt() const
  {
    switch (type_)
    {
      case ValueType::INT:
        return boost::get<int32_t>(value_);
        break;
      case ValueType::REAL:
        return static_cast<int32_t>(boost::get<double>(value_));
        break;
      default:
        throw std::runtime_error("Value is not convertible to int.");
    }
  }

  double Value::asDouble() const
  {
    switch (type_)
    {
      case ValueType::INT:
        return boost::get<int32_t>(value_) * 1.0f;
        break;
      case ValueType::REAL:
        return boost::get<double>(value_);
        break;
      default:
        throw std::runtime_error("Value ist kein Cabrio to double.");
    }
  }

  bool Value::asBool() const
  {
    if (type_ != ValueType::BOOL)
      throw std::runtime_error("Value not convertible to bool");
    return boost::get<bool>(value_);
  }

  const std::string Value::asString() const
  {
    if (type_ != ValueType::STRING)
      throw std::runtime_error("Value not convertible to string!");
    return boost::get<std::string>(value_);
  }

  Value::valuesList_t::size_type Value::size() const
  {
    if (type_ != ValueType::ARRAY)
      throw std::runtime_error("Uni::Value::size() "
                               "is only useful for arrays (value was not)");
    return boost::get<valuesList_t>(value_).size();
  }

  void Value::reserve(valuesList_t::size_type size)
  {
    if (type_ != ValueType::ARRAY)
      throw std::runtime_error("Uni::Value::reserve() "
                               "is only useful for arrays (value was not)");
    boost::get<valuesList_t>(value_).reserve(size);
  }

  bool Value::hasProperty(const std::string& key) const
  {
    if (type_ != ValueType::OBJECT)
      throw std::runtime_error("Uni::Value::hasProperty() "
                               "is only useful for objects (value was not)");
    return boost::get<valuesMap_t>(value_).count(key) > 0;
  }

  Value::valuesMap_t::const_iterator Value::objectBegin() const
  {
    if (type_ != ValueType::OBJECT)
      throw std::runtime_error("Uni::Value::objectBegin() "
                               "is only useful for objects (value was not)");
    return boost::get<valuesMap_t>(value_).begin();
  }

  Value::valuesMap_t::iterator Value::objectBegin()
  {
    if (type_ != ValueType::OBJECT)
      throw std::runtime_error("Uni::Value::objectBegin() "
                               "is only useful for objects (value was not)");
    return boost::get<valuesMap_t>(value_).begin();
  }

  Value::valuesMap_t::const_iterator Value::objectEnd() const
  {
    if (type_ != ValueType::OBJECT)
      throw std::runtime_error("Uni::Value::objectEnd() "
                               "is only useful for objects (value was not)");
    return boost::get<valuesMap_t>(value_).end();
  }

  Value::valuesMap_t::iterator Value::objectEnd()
  {
    if (type_ != ValueType::OBJECT)
      throw std::runtime_error("Uni::Value::objectEnd() "
                               "is only useful for objects (value was not)");
    return boost::get<valuesMap_t>(value_).end();
  }

  Value::valuesList_t::const_iterator Value::listBegin() const
  {
    if (type_ != ValueType::ARRAY)
      throw std::runtime_error("Uni::Value::listBegin() "
                               "is only useful for arrays (value was not)");
    return boost::get<valuesList_t>(value_).begin();
  }

  Value::valuesList_t::iterator Value::listBegin()
  {
    if (type_ != ValueType::ARRAY)
      throw std::runtime_error("Uni::Value::listBegin() "
                               "is only useful for arrays (value was not)");
    return boost::get<valuesList_t>(value_).begin();
  }

  Value::valuesList_t::const_iterator Value::listEnd() const
  {
    if (type_ != ValueType::ARRAY)
      throw std::runtime_error("Uni::Value::listEnd() "
                               "is only useful for arrays (value was not)");
    return boost::get<valuesList_t>(value_).end();
  }

  Value::valuesList_t::iterator Value::listEnd()
  {
    if (type_ != ValueType::ARRAY)
      throw std::runtime_error("Uni::Value::listEnd() "
                               "is only useful for arrays (value was not)");
    return boost::get<valuesList_t>(value_).end();
  }
}
