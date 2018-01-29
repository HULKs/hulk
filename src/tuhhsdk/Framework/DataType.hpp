#pragma once

#include <Tools/Storage/UniValue/UniConvertible.hpp>

class DataTypeBase : public Uni::To, public Uni::From {
public:
  /**
   * @brief ~DataTypeBase virtual destructor for polymorphism
   */
  virtual ~DataTypeBase() = default;
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
   * @brief reset should set the datum to a defined state
   */
  virtual void reset() = 0;

  /**
   * @brief toValue converts the DataType to an Uni::Value. This is to be
   * implemented in derived classes.
   * @param value a reference to a Uni::Value that will be filled with
   * the Uni::Value representation of the DataType
   */
  virtual void toValue(Uni::Value& /*value*/) const = 0;

  /**
   * @brief fromValue sets the DataType from an Uni::Value. This is to be
   * implemented in derived classes
   * @param value the value to set the DataType's values from
   */
  virtual void fromValue(const Uni::Value& /*value*/) = 0;
};

template<typename Derived, typename Base = DataTypeBase>
class DataType : public Base {
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
};
