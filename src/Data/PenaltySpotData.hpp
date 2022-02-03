#pragma once

#include <vector>

#include "Data/ImageSegments.hpp"
#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"
#include "Tools/Math/Eigen.hpp"

struct PenaltySpot : public Uni::From, public Uni::To
{
  PenaltySpot() = default;

  explicit PenaltySpot(const Vector2i pixelPosition)
    : pixelPosition(pixelPosition)
  {
  }

  /**
   * calculates penalty spot bounding box
   */
  void toRectangle(Rectangle<int>& rectangle)
  {
    rectangle.topLeft.x() = pixelPosition.x() - width / 2;
    rectangle.topLeft.y() = pixelPosition.y() - height / 2;
    rectangle.bottomRight.x() = pixelPosition.x() + width / 2;
    rectangle.bottomRight.y() = pixelPosition.y() + height / 2;
  }

  /**
   * @brief adds penalty spots bounding box points to convex polygon in anti clockwise order
   */
  void toConvexPolygon(ConvexPolygon<int>& convexPolygon)
  {
    // Top left
    convexPolygon.points.emplace_back(pixelPosition.x() - width / 2,
                                      pixelPosition.y() - height / 2);
    // Bottom left
    convexPolygon.points.emplace_back(pixelPosition.x() - width / 2,
                                      pixelPosition.y() + height / 2);
    // Bottom right
    convexPolygon.points.emplace_back(pixelPosition.x() + width / 2,
                                      pixelPosition.y() + height / 2);
    // Top right
    convexPolygon.points.emplace_back(pixelPosition.x() + width / 2,
                                      pixelPosition.y() - height / 2);
  }

  /**
   * @brief fromValue converts a Uni::Value to this
   * @param value the value that should be converted to this class
   */
  void fromValue(const Uni::Value& value) override
  {
    assert(value.type() == Uni::ValueType::OBJECT);
    value["relativePosition"] >> relativePosition;
    value["pixelPosition"] >> pixelPosition;
    value["width"] >> width;
    value["height"] >> height;
    value["expectedRadius"] >> expectedRadius;
    value["score"] >> score;
    value["debugPoints"] >> debugPoints;
  }

  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the value that this class should be converted to
   */
  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["relativePosition"] << relativePosition;
    value["pixelPosition"] << pixelPosition;
    value["width"] << width;
    value["height"] << height;
    value["expectedRadius"] << expectedRadius;
    value["score"] << score;
    value["debugPoints"] << debugPoints;
  }

  // the position of the penalty spot relative to the robot
  Vector2f relativePosition;
  // the position of the penalty spot in pixel coordinates
  Vector2i pixelPosition;
  // horizontal segment
  const Segment* hSegment;
  // vertical segment
  const Segment* vSegment;
  // width in pixel coordinates
  unsigned int width;
  // height in pixel coordinates
  unsigned int height;
  // the expected pixel size at that position in x direction (422)
  unsigned int expectedRadius;
  // score of the penalty spot
  float score;
  /// the sample points of the detected penalty spot
  VecVector2i debugPoints;
};

class PenaltySpotData : public DataType<PenaltySpotData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"PenaltySpotData"};
  /// the actual penalty spot datum
  PenaltySpot penaltySpot;
  /// the timestamp of the image in which it was seen
  Clock::time_point timestamp;
  /// whether the penalty spot is valid
  bool valid = false;

  /**
   * @brief reset invalidates the penalty spot
   */
  void reset() override
  {
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["penaltySpot"] << penaltySpot;
    value["timestamp"] << timestamp;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["penaltySpot"] >> penaltySpot;
    value["timestamp"] >> timestamp;
    value["valid"] >> valid;
  }
};
