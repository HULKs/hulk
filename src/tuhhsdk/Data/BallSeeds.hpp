#pragma once

#include <vector>

#include "Framework/DataType.hpp"
#include "Tools/Math/Circle.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Storage/Image.hpp"
#include "Tools/Storage/ObjectCandidate.hpp"
#include "Tools/Time.hpp"


class BallSeeds : public DataType<BallSeeds>
{
public:
  /// the name of this DataType
  DataTypeName name = "BallSeeds";

  struct Seed : public Uni::From, public Uni::To
  {
    Seed() = default;

    Seed(Vector2i position, float radius)
      : position(std::move(position))
      , radius(radius)
    {
    }
    /// the 422 seed position in the image
    Vector2i position{Vector2i::Zero()};
    /// the radius the ball would have at this seed position
    float radius{0.f};

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["position"] << position;
      value["radius"] << radius;
    }

    void fromValue(const Uni::Value& value) override
    {
      value["position"] >> position;
      value["radius"] >> radius;
    }
  };

  /// detected seeds
  std::vector<Seed> seeds;

  /// whether the ball candidates are valid
  bool valid = false;

  /**
   * @brief invalidates the position
   */
  void reset() override
  {
    valid = false;
    seeds.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["seeds"] << seeds;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["valid"] >> valid;
    value["seeds"] >> seeds;
  }
};

