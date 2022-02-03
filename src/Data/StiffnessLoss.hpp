#pragma once

#include "Framework/DataType.hpp"

class StiffnessLoss : public DataType<StiffnessLoss>
{
public:
  DataTypeName name__{"StiffnessLoss"};

  bool stiffnessLoss = false;
  bool valid = false;

  virtual void reset() override
  {
    stiffnessLoss = false;
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["stiffnessLoss"] << stiffnessLoss;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["stiffnessLoss"] >> stiffnessLoss;
    value["valid"] >> valid;
  }
};
