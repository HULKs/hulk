#pragma once

#include <Tools/Storage/UniValue/UniValue.h>

#define DataTypeName static constexpr const char*

class DataTypeBase : public Uni::To, public Uni::From
{
public:
  /**
   * @brief ~DataTypeBase virtual destructor for polymorphism
   */
  ~DataTypeBase() override = default;
  /**
   * @brief copy creates a copy of the datum
   */
  virtual DataTypeBase* copy() const = 0;
  /**
   * @brief copy copies the data to the specified place
   * @param p destination to copy the data to
   */
  virtual void copy(DataTypeBase* p) const = 0;
  /**
   * @brief reset sets the datum to a defined state
   */
  virtual void reset() = 0;
  /**
   * @brief returns the name of this data type
   * @return the name of this data type
   */
  virtual const char* getName() = 0;
  /**
   * @brief toValue converts the DataType to an Uni::Value. This is to be
   * implemented in derived classes.
   * @param value a reference to a Uni::Value that will be filled with
   * the Uni::Value representation of the DataType
   */
  void toValue(Uni::Value& /*value*/) const override = 0;
  /**
   * @brief fromValue sets the DataType from an Uni::Value. This is to be
   * implemented in derived classes
   * @param value the value to set the DataType's values from
   */
  void fromValue(const Uni::Value& /*value*/) override = 0;
  /// whether this DataType should automatically being sent via debug (if subscribed)
  bool autoUpdateDebug = true;
};

template <typename Derived, typename Base = DataTypeBase>
class DataType : public Base
{
public:
  /**
   * @brief copy creates a copy of a data type
   * @return a newly allocated copy of the data type
   */
  DataTypeBase* copy() const
  {
    return new Derived(static_cast<const Derived&>(*this));
  }
  /**
   * @brief copy creates a copy of a data type at a specified location
   * @param p destination of the copy operation
   */
  void copy(DataTypeBase* p) const
  {
    *static_cast<Derived*>(p) = Derived(static_cast<const Derived&>(*this));
  }
  /**
   * @brief returns the name of this DataType
   * @return the name of this DataType
   */
  const char* getName()
  {
    return Derived::name;
  }
};
