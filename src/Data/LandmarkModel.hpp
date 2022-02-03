#pragma once

#include <set>
#include <vector>

#include "Data/LineData.hpp"
#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Line.hpp"


class LandmarkModel : public DataType<LandmarkModel>
{
public:
  /// the name of this DataType
  DataTypeName name__{"LandmarkModel"};
  /**
   * @brief Goal stores to posts at once
   */
  struct Goal : public Uni::To, public Uni::From
  {
    /**
     * @brief Goal constructs a Goal from two posts
     */
    Goal()
    {
      left = {0, 0};
      right = {0, 0};
    }

    /**
     * @brief Goal constructs a Goal from two posts
     * @param left the left post of the goal
     * @param right the right post of the goal
     */
    Goal(const Vector2f& left, const Vector2f& right)
      : left(left)
      , right(right)
    {
    }
    /// relative position of the left post
    Vector2f left;
    /// relative position of the right post
    Vector2f right;

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["left"] << left;
      value["right"] << right;
    }

    void fromValue(const Uni::Value& value) override
    {
      value["left"] >> left;
      value["right"] >> right;
    }
  };

  struct CenterCircle : public Uni::To, public Uni::From
  {
    /**
     * @brief CenterCircle constructs an empty CenterCircle object
     */
    CenterCircle()
    {
      position = {0, 0};
      hasOrientation = false;
      orientation = 0.f;
      usedLineIds = {};
    }
    /**
     * @brief Goal constructs a CenterCircle
     * @param left the left post of the goal
     * @param right the right post of the goal
     */
    CenterCircle(const Vector2f& position, const bool hasOrientation, const float& orientation,
                 const std::vector<size_t>& usedLineIds)
      : position(position)
      , hasOrientation(hasOrientation)
      , orientation(orientation)
      , usedLineIds(usedLineIds)
    {
    }
    /// relative position of the center circle
    Vector2f position;
    /// the bool signifies whether the orientation is valid, float is the
    /// angle between the nao's x axis and the long line through the center of the circle
    bool hasOrientation;
    float orientation;
    /// all line Ids that were used to create this center circle
    /// these correspond to the indicies of the lines in LineData.lines
    std::vector<size_t> usedLineIds;

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["position"] << position;
      value["hasOrientation"] << hasOrientation;
      value["orientation"] << orientation;
      value["usedLineIds"] << usedLineIds;
    }

    void fromValue(const Uni::Value& value) override
    {
      value["position"] >> position;
      value["hasOrientation"] >> hasOrientation;
      value["orientation"] >> orientation;
      value["usedLineIds"] >> usedLineIds;
    }
  };

  struct PenaltyArea : public Uni::To, public Uni::From
  {
    PenaltyArea()
    {
      position = {0.f, 0.f};
      hasOrientation = false;
      orientation = 0.f;
      usedLineIds = {};
    }

    PenaltyArea(const Vector2f& position, const bool hasOrientation, const float orientation,
                const std::vector<size_t>& usedLineIds)
      : position(position)
      , hasOrientation(hasOrientation)
      , orientation(orientation)
      , usedLineIds(usedLineIds)
    {
    }

    /// the position of the penalty area, defined by the penalty spot position
    Vector2f position;
    /// whether the penalty area has an orientation
    bool hasOrientation;
    /// the value of the orientation in radians
    float orientation;
    /// contains the line used for orientation calculation
    std::vector<size_t> usedLineIds;

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["position"] << position;
      value["hasOrientation"] << hasOrientation;
      value["orientation"] << orientation;
      value["usedLineIds"] << usedLineIds;
    }

    void fromValue(const Uni::Value& value) override
    {
      value["position"] >> position;
      value["hasOrientation"] >> hasOrientation;
      value["orientation"] >> orientation;
      value["usedLineIds"] >> usedLineIds;
    }
  };

  struct Intersection : public Uni::To, public Uni::From
  {
    // TODO: comments
    enum class IntersectionType
    {
      UNDEFINED,
      XINTERSECTION,
      TINTERSECTION,
      LINTERSECTION
    };

    Intersection()
    {
      intersectionType = IntersectionType::UNDEFINED;
      intersectionOnLine1 = false;
      intersectionOnLine2 = false;
      position = {0, 0};
      hasOrientation = false;
      orientation = 0.f;
      usedLineIds = {};
    }

    IntersectionType intersectionType;
    bool intersectionOnLine1;
    bool intersectionOnLine2;
    Vector2f position;
    bool hasOrientation;
    float orientation;
    std::vector<size_t> usedLineIds;

    Intersection(const IntersectionType intersectionType, const bool intersectionOnLine1,
                 const bool intersectionOnLine2, const Vector2f& position,
                 const bool hasOrientation, const float orientation,
                 const std::vector<size_t>& usedLineIds)
      : intersectionType(intersectionType)
      , intersectionOnLine1(intersectionOnLine1)
      , intersectionOnLine2(intersectionOnLine2)
      , position(position)
      , hasOrientation(hasOrientation)
      , orientation(orientation)
      , usedLineIds(usedLineIds)
    {
    }

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["position"] << position;
      value["intersectionType"] << static_cast<int>(intersectionType);
      value["hasOrientation"] << hasOrientation;
      value["orientation"] << orientation;
      value["usedLineIds"] << usedLineIds;
    }

    void fromValue(const Uni::Value& value) override
    {
      value["position"] >> position;
      int intersectionTypeInt = 0;
      value["intersectionType"] >> intersectionTypeInt;
      intersectionType = static_cast<IntersectionType>(intersectionTypeInt);
      value["hasOrientation"] >> hasOrientation;
      value["orientation"] >> orientation;
      value["usedLineIds"] >> usedLineIds;
    }
  };

  /// a vector of complete goals
  std::vector<Goal> goals;
  /// a vector of center circles
  std::vector<CenterCircle> centerCircles;
  /// a vector of penalty areas
  std::vector<PenaltyArea> penaltyAreas;
  // a vector of intersections
  std::vector<Intersection> intersections;
  /// all lines after filtering
  std::vector<Line<float>> filteredLines;
  /// all line infos after filtering
  std::vector<LineInfo> filteredLineInfos;
  /// the distance threshold that was used for filtering the lines
  float maxLineProjectionDistance = 0.f;
  /// the timestamp of the image in which the landmarks were seen
  Clock::time_point timestamp;
  /**
   * @brief reset clears all vectors
   */
  void reset() override
  {
    goals.clear();
    centerCircles.clear();
    penaltyAreas.clear();
    intersections.clear();
    filteredLines.clear();
    filteredLineInfos.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["goals"] << goals;
    value["centerCircles"] << centerCircles;
    value["penaltyAreas"] << penaltyAreas;
    value["intersections"] << intersections;
    value["filteredLines"] << filteredLines;
    value["filteredLineInfos"] << filteredLineInfos;
    value["timestamp"] << timestamp;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["goals"] >> goals;
    value["centerCircles"] >> centerCircles;
    value["penaltyAreas"] >> penaltyAreas;
    value["intersections"] >> intersections;
    value["filteredLines"] >> filteredLines;
    value["filteredLineInfos"] >> filteredLineInfos;
    value["timestamp"] >> timestamp;
  }
};
