#pragma once

#include "Framework/DataType.hpp"


class PointOfInterests : public DataType<PointOfInterests>
{
public:
  struct PointOfInterest : public Uni::To, public Uni::From
  {
    PointOfInterest() = default;

    PointOfInterest(const Vector2f& pos, const float& w)
      : position(pos)
      , weight(w)
    {
    }

    PointOfInterest(const float& x, const float& y, const float& w)
      : position(x, y)
      , weight(w)
    {
    }

    Vector2f position{0.f, 0.f};
    float weight{0.f};

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["position"] << position;
      value["weight"] << weight;
    }
    void fromValue(const Uni::Value& value) override
    {
      value["position"] >> position;
      value["weight"] >> weight;
    }
  };
  /// the name of this DataType
  DataTypeName name__{"PointOfInterests"};

  /// the vector of all absolute positions of POIs
  std::vector<PointOfInterest> absolutePOIs;
  /// the most visible POI in relative coordinates
  PointOfInterest bestRelativePOI;

  bool valid = false;

  /**
   * @brief reset values to invalid value.
   */
  void reset() override
  {
    bestRelativePOI = PointOfInterest();
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["absolutePOIs"] << absolutePOIs;
    value["bestRelativePOI"] << bestRelativePOI;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["absolutePOIs"] >> absolutePOIs;
    value["bestRelativePOI"] >> bestRelativePOI;
    value["valid"] >> valid;
  }
};
