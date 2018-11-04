#pragma once

#include <Framework/DataType.hpp>

class StandUpResult : public DataType<StandUpResult> {
public:
  /// the name of this DataType
  DataTypeName name = "StandUpResult";
  /// whether a stand up finished successfully (only true for one cycle)
  bool finishedSuccessfully;
  /**
   * @brief reset resets to a default state
   */
  void reset()
  {
    finishedSuccessfully = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["finishedSuccessfully"] << finishedSuccessfully;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["finishedSuccessfully"] >> finishedSuccessfully;
  }
};
