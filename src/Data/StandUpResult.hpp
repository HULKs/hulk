#pragma once

#include "Framework/DataType.hpp"

class StandUpResult : public DataType<StandUpResult>
{
public:
  /// the name of this DataType
  DataTypeName name__{"StandUpResult"};
  /// whether a stand up finished successfully (only true for one cycle)
  bool finishedSuccessfully;
  /**
   * @brief reset resets to a default state
   */
  void reset() override
  {
    finishedSuccessfully = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["finishedSuccessfully"] << finishedSuccessfully;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["finishedSuccessfully"] >> finishedSuccessfully;
  }
};
