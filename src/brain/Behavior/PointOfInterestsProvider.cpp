#include "PointOfInterestsProvider.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"


PointOfInterestsProvider::PointOfInterestsProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , fieldDimensions_(*this)
  , robotPosition_(*this)
  , centerCircleWeight_(*this, "centerCircleWeight", [this] { fillAbsolutePOIs(); })
  , penaltyAreaWeight_(*this, "penaltyAreaWeight", [this] { fillAbsolutePOIs(); })
  , tIntersectionCenterLineWeight_(*this, "tIntersectionCenterLineWeight",
                                   [this] { fillAbsolutePOIs(); })
  , penaltyAreaCornerWeight_(*this, "penaltyAreaCornerWeight", [this] { fillAbsolutePOIs(); })
  , cornerWeight_(*this, "cornerWeight", [this] { fillAbsolutePOIs(); })
  , maxPOIDistance_(*this, "maxPOIDistance", [] {})
  , maxPOIAngle_(*this, "maxPOIAngle", [this] { maxPOIAngle_() *= TO_RAD; })
  , pointOfInterests_(*this)
{
  maxPOIAngle_() *= TO_RAD;
  fillAbsolutePOIs();
}


void PointOfInterestsProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");
  findBestPOI();
}

void PointOfInterestsProvider::fillAbsolutePOIs()
{
  pointOfInterests_->absolutePOIs.clear();

  // center circle
  pointOfInterests_->absolutePOIs.emplace_back(0.f, 0.f, centerCircleWeight_());

  // calculate the center between penalty spot and penalty area
  const float penaltyAreaDistance =
      (fieldDimensions_->fieldPenaltyAreaLength + fieldDimensions_->fieldPenaltyMarkerDistance) /
      2.f;
  // own penaltyArea
  pointOfInterests_->absolutePOIs.emplace_back(
      -fieldDimensions_->fieldLength / 2.f + penaltyAreaDistance, 0.f, penaltyAreaWeight_());

  pointOfInterests_->absolutePOIs.emplace_back(
      fieldDimensions_->fieldLength / 2.f - penaltyAreaDistance, 0.f, penaltyAreaWeight_());

  // T-intersections at center line
  pointOfInterests_->absolutePOIs.emplace_back(0.f, fieldDimensions_->fieldWidth / 2.f,
                                               tIntersectionCenterLineWeight_());
  pointOfInterests_->absolutePOIs.emplace_back(0.f, -fieldDimensions_->fieldWidth / 2.f,
                                               tIntersectionCenterLineWeight_());

  // corners at penaltyArea
  pointOfInterests_->absolutePOIs.emplace_back(
      -fieldDimensions_->fieldLength / 2.f + fieldDimensions_->fieldPenaltyAreaLength,
      fieldDimensions_->fieldPenaltyAreaWidth / 2.f, penaltyAreaCornerWeight_());
  pointOfInterests_->absolutePOIs.emplace_back(
      -fieldDimensions_->fieldLength / 2.f + fieldDimensions_->fieldPenaltyAreaLength,
      -fieldDimensions_->fieldPenaltyAreaWidth / 2.f, penaltyAreaCornerWeight_());
  pointOfInterests_->absolutePOIs.emplace_back(
      fieldDimensions_->fieldLength / 2.f - fieldDimensions_->fieldPenaltyAreaLength,
      fieldDimensions_->fieldPenaltyAreaWidth / 2.f, penaltyAreaCornerWeight_());
  pointOfInterests_->absolutePOIs.emplace_back(
      fieldDimensions_->fieldLength / 2.f - fieldDimensions_->fieldPenaltyAreaLength,
      -fieldDimensions_->fieldPenaltyAreaWidth / 2.f, penaltyAreaCornerWeight_());

  // corners
  pointOfInterests_->absolutePOIs.emplace_back(-fieldDimensions_->fieldLength / 2.f,
                                               fieldDimensions_->fieldWidth / 2.f, cornerWeight_());
  pointOfInterests_->absolutePOIs.emplace_back(
      -fieldDimensions_->fieldLength / 2.f, -fieldDimensions_->fieldWidth / 2.f, cornerWeight_());
  pointOfInterests_->absolutePOIs.emplace_back(fieldDimensions_->fieldLength / 2.f,
                                               fieldDimensions_->fieldWidth / 2.f, cornerWeight_());
  pointOfInterests_->absolutePOIs.emplace_back(
      fieldDimensions_->fieldLength / 2.f, -fieldDimensions_->fieldWidth / 2.f, cornerWeight_());
}

void PointOfInterestsProvider::findBestPOI()
{
  for (const auto& absPOI : pointOfInterests_->absolutePOIs)
  {
    const Vector2f relativePosition = robotPosition_->fieldToRobot(absPOI.position);
    const float angleToPOI = std::atan2(relativePosition.y(), relativePosition.x());
    const float distanceToPOI = relativePosition.norm();
    if (std::abs(angleToPOI) < maxPOIAngle_() && distanceToPOI < maxPOIDistance_() &&
        absPOI.weight > pointOfInterests_->bestRelativePOI.weight)
    {
      pointOfInterests_->bestRelativePOI =
          PointOfInterests::PointOfInterest(relativePosition, absPOI.weight);
      pointOfInterests_->valid = true;
    }
  }
}
