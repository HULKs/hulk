#pragma once

#include <array>
#include <cstdint>
#include <iostream>
#include <list>
#include <map>
#include <memory>
#include <set>
#include <stdexcept>
#include <string>
#include <variant>
#include <vector>

#include "Tools/Math/Eigen.hpp"

namespace Uni
{
  enum class ValueType : uint8_t
  {
    NIL,
    INT32,
    INT64,
    REAL,
    BOOL,
    STRING,
    ARRAY,
    OBJECT
  };

  // Forward declaration
  class Value;

  class To
  {
  public:
    /**
     * @brief ~To virtual destructor for polymorphism
     */
    virtual ~To() = default;
    /**
     * @brief toValue converts a data structure to a Uni::Value
     * @param value the new Uni::Value
     */
    virtual void toValue(Value& value) const = 0;
  };

  class From
  {
  public:
    /**
     * @brief ~From virtual destructor for polymorphism
     */
    virtual ~From() = default;
    /**
     * @brief fromValue converts a Uni::Value to a data structure
     * @param value Uni::Value that is to be converted
     */
    virtual void fromValue(const Value& value) = 0;
  };

  class Value
  {
  public:
    using valuesStringMap_t = std::map<std::string, Value>;
    using valuesVector_t = std::vector<Value>;

    /**
     * @brief default constructor creating a Uni::Value of Uni::ValueType t
     * @param t value type to be constructed
     */
    Value(ValueType t = ValueType::NIL);
    /**
     * @brief default copy contructor
     * @param other reference to the other value to copy from
     */
    Value(const Value& other) = default;
    /**
     * @brief default move contructor
     * @param other reference to the other value to be moved
     */
    Value(Value&& other) = default;
    /**
     * @brief copy assignment operator copies rhs to this Uni::Value if ValueType is not NIL
     * @param rhs the value to be assigned
     * @return reference to the new value
     */
    Value& operator=(const Value& rhs) = default;
    /**
     * @brief move assignment operator copies rhs to this Uni::Value if ValueType is not NIL
     * @param rhs the value to be assigned
     * @return reference to the new value
     */
    Value& operator=(Value&& rhs) = default;
    /**
     * @brief constructor creating a Uni::Value of Uni::ValueType INT32
     * @param i value to be constructed
     */
    Value(int32_t i);
    /**
     * @brief constructor creating a Uni::Value of Uni::ValueType INT64
     * @param i value to be constructed
     */
    Value(int64_t i);
    /**
     * @brief constructor creating a Uni::Value of Uni::ValueType REAL
     * @param d value to be constructed
     */
    Value(double d);
    /**
     * @brief constructor creating a Uni::Value of Uni::ValueType BOOL
     * @param b value to be constructed
     */
    Value(bool b);
    /**
     * @brief constructor creating a Uni::Value of Uni::ValueType STRING
     * @param s value to be constructed
     */
    explicit Value(const std::string& s);
    /**
     * @brief constructor creating a Uni::Value of Uni::ValueType STRING
     * @param s value to be constructed
     */
    explicit Value(const char* s);
    /**
     * @brief constructor creating a Uni::Value from a Uni::To object
     * @param to reference to the Uni::To object
     */
    Value(const To& to);

    /**
     * @brief operator[] accesses Uni::Value of Uni::ValueType OBJECT at position key. If Value is
     * empty creates a new Value of Type OBJECT. Performs insertion if such key does not exist.
     * @param key the position to be accessed
     * @throws std::runtime_error when accessing Uni::Value of Uni::ValueType other than OBJECT
     * @return reference to the Value at accessed position
     */
    Value& operator[](const char* key);
    /**
     * @brief operator[] accesses Uni::Value of Uni::ValueType OBJECT at position key
     * @param key the position to be accessed
     * @return reference to the Value at accessed position
     */
    const Value& operator[](const char* key) const;
    /**
     * @see Uni::Value::operator[](const char* key)
     */
    Value& operator[](const std::string& key);
    /**
     * @see Uni::Value::operator[](const char* key) const
     */
    const Value& operator[](const std::string& key) const;
    /**
     * @brief operator[] accesses Uni::Value of Uni::ValueType ARRAY at position pos. If Value is
     * empty creates a new Value of Type ARRAY. If pos is out of range the vector will be resized
     * accordingly.
     * @param pos the position to be accessed
     * @throws std::runtime_error when accessing Uni::Value of Uni::ValueType other than ARRAY
     * @return reference to the Uni::Value at accessed position
     */
    Value& operator[](valuesVector_t::size_type pos);
    /**
     * @see Uni::Value::operator[valuesVector_t::size_type)
     */
    Value& at(valuesVector_t::size_type pos);
    /**
     * @brief at accesses Uni::Value of Uni::ValueType ARRAY at position pos
     * @param pos the position to be accessed
     * @throws std::out_of_range when pos is out of range
     * @return reference to the Uni::Value at accessed position
     */
    const Value& at(valuesVector_t::size_type pos) const;
    /**
     * @see Uni::Value::at(valuesVector_t::size_type) const
     */
    const Value& operator[](valuesVector_t::size_type pos) const;

    /**
     * @brief type returns the Uni::ValueType of this Uni::Value
     * @return type Uni::ValueType
     */
    ValueType type() const;

    /**
     * @brief asInt32() returns value converted to int32_t
     * @throws std::runtime_error if value is not convertible to int32_t
     * @return value as int32_t
     */
    int32_t asInt32() const;
    /**
     * @brief asInt64() returns value converted to int64_t
     * @throws std::runtime_error if value is not convertible to int64_t
     * @return value as int64_t
     */
    int64_t asInt64() const;
    /**
     * @brief asDouble() returns value converted to double
     * @throws std::runtime_error if value is not convertible to double
     * @return value as double
     */
    double asDouble() const;
    /**
     * @brief asBool() returns value converted to bool
     * @throws std::runtime_error if value is not convertible to bool
     * @return value as bool
     */
    bool asBool() const;
    /**
     * @brief asString() returns value converted to std::string
     * @throws std::runtime_error if value is not convertible to std::string
     * @return value as std::string
     */
    std::string asString() const;

    /**
     * @brief clear() clears the values of this Uni::Value if ValueType is OBJECT or ARRAY
     */
    void clear();

    /**
     * @brief size() returns the number of elements in this Uni::Value of ValueType ARRAY OR OBJECT
     * @return number of elements
     */
    std::size_t size() const;

    /**
     * @brief reserve() reserves storage
     * @param size new capacity of the vector
     */
    void reserve(valuesVector_t::size_type size);
    /**
     * @brief contains() checks if the container contains element with specific key
     * @param key key of the element to search for
     * @return true if there is such an element, otherwise false
     */
    bool contains(const std::string&) const;

    /**
     * @brief objectBegin() returns an iterator to the first element of the Uni::Value OBJECT
     * @return iterator referencing the first element
     */
    valuesStringMap_t::iterator objectBegin();
    /**
     * @brief objectEnd() returns an iterator past-the-last element of the Uni::Value OBJECT
     * @return iterator past-the-last element
     */
    valuesStringMap_t::iterator objectEnd();
    /**
     * @brief vectorBegin() returns an iterator to the first element of the Uni::Value ARRAY
     * @return iterator referencing the first element
     */
    valuesVector_t::iterator vectorBegin();
    /**
     * @brief vectorEnd() returns an iterator past-the-last element of the Uni::Value ARRAY
     * @return iterator past-the-last element
     */
    valuesVector_t::iterator vectorEnd();

    /**
     * @brief constant equivalent of objectBegin()
     * @see objectBegin()
     */
    valuesStringMap_t::const_iterator objectBegin() const;
    /**
     * @brief constant equivalent of objectEnd()
     * @see objectEnd()
     */
    valuesStringMap_t::const_iterator objectEnd() const;
    /**
     * @brief constant equivalent of vectorBegin()
     * @see vectorBegin()
     */
    valuesVector_t::const_iterator vectorBegin() const;
    /**
     * @brief constant equivalent of vectorEnd()
     * @see vectorEnd()
     */
    valuesVector_t::const_iterator vectorEnd() const;


  private:
    using value_t = std::variant<int64_t, int32_t, double, bool, std::string, valuesStringMap_t,
                                 valuesVector_t>;
    value_t value_;
    ValueType type_;
  };
} // namespace Uni

inline void operator>>(const Uni::Value& in, Uni::Value& out)
{
  out = in;
}

inline void operator<<(Uni::Value& out, const Uni::Value& in)
{
  out = in;
}

inline void operator>>(const Uni::Value& in, int32_t& out)
{
  out = in.asInt32();
}

inline void operator<<(Uni::Value& out, const int32_t in)
{
  out = Uni::Value(in);
}

inline void operator>>(const Uni::Value& in, uint32_t& out)
{
  out = static_cast<uint32_t>(in.asInt64());
}

inline void operator<<(Uni::Value& out, const uint32_t in)
{
  out = Uni::Value((int64_t)in);
}

inline void operator>>(const Uni::Value& in, int64_t& out)
{
  out = in.asInt64();
}

inline void operator<<(Uni::Value& out, const int64_t in)
{
  out = Uni::Value(in);
}

inline void operator>>(const Uni::Value& in, uint64_t& out)
{
  out = static_cast<uint64_t>(in.asInt64());
}

inline void operator<<(Uni::Value& out, const uint64_t in)
{
  out = Uni::Value((int64_t)in);
}

#if defined __APPLE__
inline void operator>>(const Uni::Value& in, size_t& out)
{
  out = in.asInt64();
}

inline void operator<<(Uni::Value& out, const size_t in)
{
  out = Uni::Value((int64_t)in);
}
#endif

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

inline void operator<<(Uni::Value& out, const Uni::To& in)
{
  in.toValue(out);
}

inline void operator>>(const Uni::Value& in, Uni::From& out)
{
  out.fromValue(in);
}

#include "EigenStreaming.hpp"
#include "UniValueStreaming.hpp"
