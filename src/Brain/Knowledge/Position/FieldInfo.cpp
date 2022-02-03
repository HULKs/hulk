#include "Brain/Knowledge/Position/FieldInfo.hpp"
#include <cmath>

FieldInfo::FieldInfo(const PlayerConfiguration& playerConfiguration,
                     const FieldDimensions& fieldDimensions)
  : playerConfiguration_(playerConfiguration)
{
  const float fieldLength = fieldDimensions.fieldLength;
  const float fieldWidth = fieldDimensions.fieldWidth;
  const float goalBoxAreaWidth = fieldDimensions.fieldGoalBoxAreaWidth;
  const float goalBoxAreaLength = fieldDimensions.fieldGoalBoxAreaLength;
  const float penaltyBoxWidth = fieldDimensions.fieldPenaltyAreaWidth;
  const float penaltyBoxLength = fieldDimensions.fieldPenaltyAreaLength;
  const float centerCircleRadius = fieldDimensions.fieldCenterCircleDiameter * 0.5f;
  // The goal post position is assumed to be in the center of the post, thus two goal post radii
  // have to be added (a.k.a one diameter).
  const float goalPostDistance = fieldDimensions.goalInnerWidth + fieldDimensions.goalPostDiameter;
  const float goalDepth = fieldDimensions.goalDepth;
  const float penaltyBoxInFieldDistance = fieldLength * 0.5f - penaltyBoxLength;
  const float goalBoxAreaInFieldDistance = fieldLength * 0.5f - goalBoxAreaLength;

  // field border
  lines.push_back(
      Line<float>{{-fieldLength / 2, fieldWidth / 2}, {fieldLength / 2, fieldWidth / 2}});
  lines.push_back(
      Line<float>{{-fieldLength / 2, -fieldWidth / 2}, {fieldLength / 2, -fieldWidth / 2}});
  lines.push_back(
      Line<float>{{-fieldLength / 2, fieldWidth / 2}, {-fieldLength / 2, -fieldWidth / 2}});
  lines.push_back(
      Line<float>{{fieldLength / 2, fieldWidth / 2}, {fieldLength / 2, -fieldWidth / 2}});

  // center line
  lines.push_back(Line<float>{{0, fieldWidth / 2}, {0, -fieldWidth / 2}});

  // center circle as polygon
  for (float phi = 0; phi < 2 * M_PI; phi += M_PI / 8)
  {
    lines.push_back(
        Line<float>{polar2cartesian({centerCircleRadius, phi}),
                    polar2cartesian({centerCircleRadius, phi + static_cast<float>(M_PI / 8)})});
  }

  // goal box area home
  lines.push_back(Line<float>{{-fieldLength / 2, goalBoxAreaWidth / 2},
                              {-goalBoxAreaInFieldDistance, goalBoxAreaWidth / 2}});
  lines.push_back(Line<float>{{-fieldLength / 2, -goalBoxAreaWidth / 2},
                              {-goalBoxAreaInFieldDistance, -goalBoxAreaWidth / 2}});

  lines.push_back(Line<float>{{-goalBoxAreaInFieldDistance, goalBoxAreaWidth / 2},
                              {-goalBoxAreaInFieldDistance, -goalBoxAreaWidth / 2}});

  // goal box area away
  lines.push_back(Line<float>{{fieldLength / 2, goalBoxAreaWidth / 2},
                              {goalBoxAreaInFieldDistance, goalBoxAreaWidth / 2}});
  lines.push_back(Line<float>{{fieldLength / 2, -goalBoxAreaWidth / 2},
                              {goalBoxAreaInFieldDistance, -goalBoxAreaWidth / 2}});

  lines.push_back(Line<float>{{goalBoxAreaInFieldDistance, goalBoxAreaWidth / 2},
                              {goalBoxAreaInFieldDistance, -goalBoxAreaWidth / 2}});

  // penalty box home
  lines.push_back(Line<float>{{-fieldLength / 2, penaltyBoxWidth / 2},
                              {-penaltyBoxInFieldDistance, penaltyBoxWidth / 2}});
  lines.push_back(Line<float>{{-fieldLength / 2, -penaltyBoxWidth / 2},
                              {-penaltyBoxInFieldDistance, -penaltyBoxWidth / 2}});

  lines.push_back(Line<float>{{-penaltyBoxInFieldDistance, penaltyBoxWidth / 2},
                              {-penaltyBoxInFieldDistance, -penaltyBoxWidth / 2}});

  // penalty box away
  lines.push_back(Line<float>{{fieldLength / 2, penaltyBoxWidth / 2},
                              {penaltyBoxInFieldDistance, penaltyBoxWidth / 2}});
  lines.push_back(Line<float>{{fieldLength / 2, -penaltyBoxWidth / 2},
                              {penaltyBoxInFieldDistance, -penaltyBoxWidth / 2}});

  lines.push_back(Line<float>{{penaltyBoxInFieldDistance, penaltyBoxWidth / 2},
                              {penaltyBoxInFieldDistance, -penaltyBoxWidth / 2}});

  // There are sometimes lines in the goal support structure.
  if (playerConfiguration_.playerNumber == 1)
  {
    lines.push_back(Line<float>{{fieldLength / 2, -goalPostDistance / 2},
                                {fieldLength / 2 + goalDepth, -goalPostDistance / 2}});
    lines.push_back(Line<float>{{fieldLength / 2, goalPostDistance / 2},
                                {fieldLength / 2 + goalDepth, goalPostDistance / 2}});
    lines.push_back(Line<float>{{fieldLength / 2 + goalDepth, -goalPostDistance / 2},
                                {fieldLength / 2 + goalDepth, goalPostDistance / 2}});

    lines.push_back(Line<float>{{-fieldLength / 2, -goalPostDistance / 2},
                                {-fieldLength / 2 - goalDepth, -goalPostDistance / 2}});
    lines.push_back(Line<float>{{-fieldLength / 2, goalPostDistance / 2},
                                {-fieldLength / 2 - goalDepth, goalPostDistance / 2}});
    lines.push_back(Line<float>{{-fieldLength / 2 - goalDepth, -goalPostDistance / 2},
                                {-fieldLength / 2 - goalDepth, goalPostDistance / 2}});
  }

  // Goals
  // home
  goalPosts.emplace_back(-fieldLength / 2, goalPostDistance / 2);
  goalPosts.emplace_back(-fieldLength / 2, -goalPostDistance / 2);
  // away
  goalPosts.emplace_back(fieldLength / 2, goalPostDistance / 2);
  goalPosts.emplace_back(fieldLength / 2, -goalPostDistance / 2);

  // Penalty spots
  // home
  penaltySpots.emplace_back(-fieldLength / 2 + fieldDimensions.fieldPenaltyMarkerDistance, 0.f);
  // away
  penaltySpots.emplace_back(fieldLength / 2 - fieldDimensions.fieldPenaltyMarkerDistance, 0.f);
}

Vector2f FieldInfo::polar2cartesian(const Vector2f& polar) const
{
  float r = polar.x();
  float phi = polar.y();
  return {r * std::cos(phi), r * std::sin(phi)};
}
