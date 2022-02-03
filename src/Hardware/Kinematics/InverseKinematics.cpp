#include "Hardware/Kinematics/InverseKinematics.hpp"
#include "Hardware/RobotMetrics.hpp"
#include "Tools/Math/Angle.hpp"
#include <cmath>

InverseKinematics::InverseKinematics(const RobotMetrics& robotMetrics)
  : robotMetrics_(robotMetrics)
{
}

float InverseKinematics::getPitchLimit(const float y, const float k) const
{
  const float ySquared = y * y;
  const float upperArmLength = robotMetrics_.link(Links::UPPER_ARM_LENGTH);
  const float upperArmLengthSquared = upperArmLength * upperArmLength;
  return k * std::sqrt(upperArmLengthSquared - ySquared);
}

JointsLegArray<float> InverseKinematics::getLLegAngles(const KinematicMatrix& desired) const
{
  // given is the desired position and orientation of the foot
  // but we need the desired position and rotation of the ankle
  // first transform to ankle space and shift about FOOT_HEIGTH
  // to get the desired ankle
  KinematicMatrix ankleInv =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::FOOT_HEIGHT)) * desired.inverted();

  // now transform back to torso space
  KinematicMatrix ankleDesired = ankleInv.inverted();

  // transformation of the desired position to the Hip Space
  KinematicMatrix ankle2hip = KinematicMatrix::transY(-robotMetrics_.link(Links::HIP_OFFSET_Y)) *
                              KinematicMatrix::transZ(robotMetrics_.link(Links::HIP_OFFSET_Z)) *
                              ankleDesired;

  // Transformation to the rotated Hip Space
  KinematicMatrix ankle2hipOrthogonal = KinematicMatrix::rotX(-45.0f * TO_RAD) * ankle2hip;

  // calculate the the distance from hip to ankle
  float l = ankle2hipOrthogonal.posV.norm();
  // normal vektor to ankle
  Vector3f n = ankle2hipOrthogonal.posV / l;

  // check wether the position is reachable
  float aKneePitch = 0.f;

  if (l > robotMetrics_.lengths().maxLegLength)
  {
    ankle2hipOrthogonal.posV = n * robotMetrics_.lengths().maxLegLength;
    l = robotMetrics_.lengths().maxLegLength;
    aKneePitch = 0.0f;
  }
  else if (l < robotMetrics_.lengths().minLegLength)
  {
    ankle2hipOrthogonal.posV = n * robotMetrics_.lengths().minLegLength;
    l = robotMetrics_.lengths().minLegLength;
    aKneePitch = robotMetrics_.maxRange(Joints::L_KNEE_PITCH);
  }
  else
  {
    // calculate the knee angle from thigh length, tibia length and hip-ankle distance
    aKneePitch = static_cast<float>(
        M_PI - std::acos((std::pow(robotMetrics_.link(Links::THIGH_LENGTH), 2) +
                          std::pow(robotMetrics_.link(Links::TIBIA_LENGTH), 2) - std::pow(l, 2)) /
                         (2.f * robotMetrics_.link(Links::THIGH_LENGTH) *
                          robotMetrics_.link(Links::TIBIA_LENGTH))));
  }

  // inverse needed
  KinematicMatrix hipOrthogonal2ankle = ankle2hipOrthogonal.inverted();

  // calculate angle for ankle pitch
  auto aAnklePitch1 = static_cast<float>(
      std::acos((std::pow(robotMetrics_.link(Links::TIBIA_LENGTH), 2) + std::pow(l, 2) -
                 std::pow(robotMetrics_.link(Links::THIGH_LENGTH), 2)) /
                (2.f * robotMetrics_.link(Links::TIBIA_LENGTH) * l)));

  Vector3f vHipAnkle = hipOrthogonal2ankle.posV;
  float aAnklePitch2 =
      std::atan2(vHipAnkle.x(), std::sqrt(std::pow(vHipAnkle.y(), 2) + std::pow(vHipAnkle.z(), 2)));

  float aAnklePitch = -(aAnklePitch1 + aAnklePitch2);

  // calculate angle for ankle roll
  float aAnkleRoll = std::atan2(vHipAnkle.y(), vHipAnkle.z());


  // transform the desired position from ankle space to hip
  KinematicMatrix thigh2Foot = KinematicMatrix::rotX(-aAnkleRoll) *
                               KinematicMatrix::rotY(-aAnklePitch) *
                               KinematicMatrix::transZ(robotMetrics_.link(Links::TIBIA_LENGTH)) *
                               KinematicMatrix::rotY(-aKneePitch) *
                               KinematicMatrix::transZ(robotMetrics_.link(Links::THIGH_LENGTH));

  // get the transformation to Hip Orthogonal
  KinematicMatrix hipOrthogonal2thigh = ankle2hipOrthogonal * thigh2Foot;

  // get angles from the transformation matrix
  auto hipRotM = hipOrthogonal2thigh.rotM.toRotationMatrix();
  float alphaX = std::asin(hipRotM(2, 1));
  float aHipYawPitch = -std::atan2(-hipRotM(0, 1), hipRotM(1, 1));
  float aHipPitch = std::atan2(-hipRotM(2, 0), hipRotM(2, 2));
  auto aHipRoll = static_cast<float>(alphaX + M_PI / 4.f);

  // constraints on angles

  // ankle Pitch
  if (aAnklePitch < robotMetrics_.minRange(Joints::L_ANKLE_PITCH))
  {
    aAnklePitch = robotMetrics_.minRange(Joints::L_ANKLE_PITCH);
  }
  else if (aAnklePitch > robotMetrics_.maxRange(Joints::L_ANKLE_PITCH))
  {
    aAnklePitch = robotMetrics_.maxRange(Joints::L_ANKLE_PITCH);
  }

  // ankleRoll
  if (aAnkleRoll < robotMetrics_.minRangeLAnkleRoll(aAnklePitch))
  {
    aAnkleRoll = robotMetrics_.minRangeLAnkleRoll(aAnklePitch);
  }
  else if (aAnkleRoll > robotMetrics_.maxRangeLAnkleRoll(aAnklePitch))
  {
    aAnkleRoll = robotMetrics_.maxRangeLAnkleRoll(aAnklePitch);
  }

  // hipYaw
  if (aHipYawPitch < robotMetrics_.minRange(Joints::L_HIP_YAW_PITCH))
  {
    aHipYawPitch = robotMetrics_.minRange(Joints::L_HIP_YAW_PITCH);
  }
  else if (aHipYawPitch > robotMetrics_.maxRange(Joints::L_HIP_YAW_PITCH))
  {
    aHipYawPitch = robotMetrics_.maxRange(Joints::L_HIP_YAW_PITCH);
  }

  // hipPitch
  if (aHipPitch < robotMetrics_.minRange(Joints::L_HIP_PITCH))
  {
    aHipPitch = robotMetrics_.minRange(Joints::L_HIP_PITCH);
  }
  else if (aHipPitch > robotMetrics_.maxRange(Joints::L_HIP_PITCH))
  {
    aHipPitch = robotMetrics_.maxRange(Joints::L_HIP_PITCH);
  }

  // hipRoll
  if (aHipRoll < robotMetrics_.minRange(Joints::L_HIP_ROLL))
  {
    aHipRoll = robotMetrics_.minRange(Joints::L_HIP_ROLL);
  }
  else if (aHipRoll > robotMetrics_.maxRange(Joints::L_HIP_ROLL))
  {
    aHipRoll = robotMetrics_.maxRange(Joints::L_HIP_ROLL);
  }

  // create angle vector
  return {{aHipYawPitch, aHipRoll, aHipPitch, aKneePitch, aAnklePitch, aAnkleRoll}};
}


JointsLegArray<float> InverseKinematics::getRLegAngles(const KinematicMatrix& desired) const
{
  // given is the desired position and orientation of the foot
  // but we need the desired position and rotation of the ankle
  // first transform to ankle space and shift about FOOT_HEIGTH
  // to get the desired ankle
  KinematicMatrix ankleInv =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::FOOT_HEIGHT)) * desired.inverted();

  // transform back to torso space
  KinematicMatrix ankleDesired = ankleInv.inverted();

  // transformation of the desired position to the Hip Space
  KinematicMatrix ankle2hip = KinematicMatrix::transY(robotMetrics_.link(Links::HIP_OFFSET_Y)) *
                              KinematicMatrix::transZ(robotMetrics_.link(Links::HIP_OFFSET_Z)) *
                              ankleDesired;

  // Transformation to the rotated Hip Space
  KinematicMatrix ankle2hipOrthogonal = KinematicMatrix::rotX(45.0f * TO_RAD) * ankle2hip;

  // calculate the the distance from hip to ankle
  float l = ankle2hipOrthogonal.posV.norm();

  // normal vector to ankle
  Vector3f n = ankle2hipOrthogonal.posV / l;

  // check wether the position is reachable
  float aKneePitch = NAN;

  if (l > robotMetrics_.lengths().maxLegLength)
  {
    ankle2hipOrthogonal.posV = n * robotMetrics_.lengths().maxLegLength;
    l = robotMetrics_.lengths().maxLegLength;
    aKneePitch = 0.0f;
  }
  else if (l < robotMetrics_.lengths().minLegLength)
  {
    ankle2hipOrthogonal.posV = n * robotMetrics_.lengths().minLegLength;
    l = robotMetrics_.lengths().minLegLength;
    aKneePitch = robotMetrics_.maxRange(Joints::R_KNEE_PITCH);
  }
  else
  {
    // calculate the knee angle from thigh length, tibia length and hip-ankle distance
    aKneePitch = static_cast<float>(
        M_PI - std::acos((pow(robotMetrics_.link(Links::THIGH_LENGTH), 2) +
                          std::pow(robotMetrics_.link(Links::TIBIA_LENGTH), 2) - std::pow(l, 2)) /
                         (2.f * robotMetrics_.link(Links::THIGH_LENGTH) *
                          robotMetrics_.link(Links::TIBIA_LENGTH))));
  }

  // inverse needed
  KinematicMatrix hipOrthogonal2ankle = ankle2hipOrthogonal.inverted();

  // calculate angle for ankle pitch
  auto aAnklePitch1 = static_cast<float>(
      std::acos((std::pow(robotMetrics_.link(Links::TIBIA_LENGTH), 2) + std::pow(l, 2) -
                 std::pow(robotMetrics_.link(Links::THIGH_LENGTH), 2)) /
                (2.f * robotMetrics_.link(Links::TIBIA_LENGTH) * l)));

  Vector3f vHipAnkle = hipOrthogonal2ankle.posV;
  float aAnklePitch2 =
      std::atan2(vHipAnkle.x(), sqrt(pow(vHipAnkle.y(), 2) + pow(vHipAnkle.z(), 2)));

  float aAnklePitch = -(aAnklePitch1 + aAnklePitch2);

  // calculate angle for ankle roll
  float aAnkleRoll = std::atan2(vHipAnkle.y(), vHipAnkle.z());


  // transform the desired position from ankle space to hip
  KinematicMatrix thigh2Foot = KinematicMatrix::rotX(-aAnkleRoll) *
                               KinematicMatrix::rotY(-aAnklePitch) *
                               KinematicMatrix::transZ(robotMetrics_.link(Links::TIBIA_LENGTH)) *
                               KinematicMatrix::rotY(-aKneePitch) *
                               KinematicMatrix::transZ(robotMetrics_.link(Links::THIGH_LENGTH));

  // get the transformation to Hip Orthogonal
  KinematicMatrix hipOrthogonal2thigh = ankle2hipOrthogonal * thigh2Foot;

  // get angles from the transformation matrix
  auto hipRotM = hipOrthogonal2thigh.rotM.toRotationMatrix();
  float alphaX = std::asin(hipRotM(2, 1));
  float aHipYawPitch = std::atan2(-hipRotM(0, 1), hipRotM(1, 1));
  float aHipPitch = std::atan2(-hipRotM(2, 0), hipRotM(2, 2));
  float aHipRoll = alphaX - static_cast<float>(M_PI) / 4.f;

  // constraints on angles

  // ankle Pitch
  if (aAnklePitch < robotMetrics_.minRange(Joints::R_ANKLE_PITCH))
  {
    aAnklePitch = robotMetrics_.minRange(Joints::R_ANKLE_PITCH);
  }
  else if (aAnklePitch > robotMetrics_.maxRange(Joints::R_ANKLE_PITCH))
  {
    aAnklePitch = robotMetrics_.maxRange(Joints::R_ANKLE_PITCH);
  }

  // ankleRoll
  if (aAnkleRoll < robotMetrics_.minRangeRAnkleRoll(aAnklePitch))
  {
    aAnkleRoll = robotMetrics_.minRangeRAnkleRoll(aAnklePitch);
  }
  else if (aAnkleRoll > robotMetrics_.maxRangeRAnkleRoll(aAnklePitch))
  {
    aAnkleRoll = robotMetrics_.maxRangeRAnkleRoll(aAnklePitch);
  }

  // hipYaw
  if (aHipYawPitch < robotMetrics_.minRange(Joints::R_HIP_YAW_PITCH))
  {
    aHipYawPitch = robotMetrics_.minRange(Joints::R_HIP_YAW_PITCH);
  }
  else if (aHipYawPitch > robotMetrics_.maxRange(Joints::R_HIP_YAW_PITCH))
  {
    aHipYawPitch = robotMetrics_.maxRange(Joints::R_HIP_YAW_PITCH);
  }

  // hipPitch
  if (aHipPitch < robotMetrics_.minRange(Joints::R_HIP_PITCH))
  {
    aHipPitch = robotMetrics_.minRange(Joints::R_HIP_PITCH);
  }
  else if (aHipPitch > robotMetrics_.maxRange(Joints::R_HIP_PITCH))
  {
    aHipPitch = robotMetrics_.maxRange(Joints::R_HIP_PITCH);
  }

  // hipRoll
  if (aHipRoll < robotMetrics_.minRange(Joints::R_HIP_ROLL))
  {
    aHipRoll = robotMetrics_.minRange(Joints::R_HIP_ROLL);
  }
  else if (aHipRoll > robotMetrics_.maxRange(Joints::R_HIP_ROLL))
  {
    aHipRoll = robotMetrics_.maxRange(Joints::R_HIP_ROLL);
  }

  // create angle vector
  return {{aHipYawPitch, aHipRoll, aHipPitch, aKneePitch, aAnklePitch, aAnkleRoll}};
}


JointsArmArray<float> InverseKinematics::getLArmAngles(const KinematicMatrix& desired,
                                                       float handOpening) const
{

  // Transformation of the desired hand position to shoulder space
  KinematicMatrix hand2Shoulder =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::SHOULDER_OFFSET_Z)) *
      KinematicMatrix::transY(-robotMetrics_.link(Links::SHOULDER_OFFSET_Y)) * desired;

  // distance from shoulder to desired hand position
  float l = hand2Shoulder.posV.norm();

  // normalized Vector from shoulder to desired hand position
  Vector3f n = hand2Shoulder.posV / l;

  // declar ElbowRoll
  float aElbowRoll = NAN;

  // check, if the desired position is reachable
  if (l > robotMetrics_.lengths().maxArmLength)
  {
    hand2Shoulder.posV = n * robotMetrics_.lengths().maxArmLength;
    l = robotMetrics_.lengths().maxArmLength;
    aElbowRoll = robotMetrics_.maxRange(Joints::L_ELBOW_ROLL);
  }
  else if (l < robotMetrics_.lengths().minArmLength)
  {
    hand2Shoulder.posV = n * robotMetrics_.lengths().minArmLength;
    l = robotMetrics_.lengths().minArmLength;
    aElbowRoll = robotMetrics_.minRange(Joints::L_ELBOW_ROLL);
  }
  else
  {
    // rule of cosines
    aElbowRoll = static_cast<float>(
        std::acos((std::pow(robotMetrics_.link(Links::UPPER_ARM_LENGTH), 2) +
                   std::pow(robotMetrics_.lengths().foreArmLength, 2) - std::pow(l, 2)) /
                  (2.f * robotMetrics_.link(Links::UPPER_ARM_LENGTH) *
                   robotMetrics_.lengths().foreArmLength)) -
        M_PI);
  }

  // calculation of the circles radius on which the elbow can be positioned
  auto beta = static_cast<float>(
      std::acos((std::pow(l, 2) + std::pow(robotMetrics_.link(Links::UPPER_ARM_LENGTH), 2) -
                 std::pow(robotMetrics_.lengths().foreArmLength, 2)) /
                (2.f * l * robotMetrics_.link(Links::UPPER_ARM_LENGTH))));

  float r = std::sin(beta) * robotMetrics_.link(Links::UPPER_ARM_LENGTH);

  // distance from shoulder to circle midpoint
  float d = std::cos(beta) * robotMetrics_.link(Links::UPPER_ARM_LENGTH);

  // Elbow position from desired hand position and orientation
  KinematicMatrix shoulder2Elbow =
      KinematicMatrix::transX(robotMetrics_.lengths().foreArmLength) * hand2Shoulder.inverted();

  KinematicMatrix elbow2Shoulder = shoulder2Elbow.inverted();

  // distance from desired elbow position to circle surface
  float s = n.dot(elbow2Shoulder.posV) - d;

  // projection of desired elbow position on circle surface
  Vector3f p = elbow2Shoulder.posV - n * s;

  // circle midpoint
  Vector3f m = n * d;

  // Vector from m to p
  Vector3f vecMP = p - m;
  vecMP.normalize();

  // calculate reachable elbow position
  Vector3f pReachable = m + vecMP * r;
  Vector3f pDesired = pReachable;

  /* calculation of rotation angles, such that the shoulder coordinate-system
   * can be transformed to the circle surface.
   * The y- and z- axes are in the surface, the x-axis is the normal vector
   */
  float a1 = std::atan2(m.y(), m.x());
  float a2 = std::atan2(m.z(), std::sqrt(std::pow(m.x(), 2) + std::pow(m.z(), 2)));

  // Transformation matrix to circle space
  KinematicMatrix toCirc = KinematicMatrix::rotZ(a1) * KinematicMatrix::rotY(-a2);

  Vector3f pToCirc = toCirc.inverted() * pReachable;

  float a3 = std::atan2(-pToCirc.y(), pToCirc.z());

  // orthogonal circle vectors
  Vector3f u = toCirc * KinematicMatrix::rotX(a3) * Vector3f(0, r, 0);
  Vector3f v = toCirc * KinematicMatrix::rotX(a3) * Vector3f(0, 0, r);

  // set step size for iteration
  int circleParts = 60;
  float step = 2.f * static_cast<float>(M_PI) / static_cast<float>(circleParts);

  // constant for shoulder roll limits
  float k = std::cos(robotMetrics_.maxRange(Joints::L_SHOULDER_PITCH));

  // iteration variables
  float t = 0;
  float bestDis = std::numeric_limits<float>::infinity();
  float bestT = t;
  bool noAvailableCirclePoint = true;
  bool optimumFound = false;

  float aShoulderRoll = 0.0f;
  float aShoulderPitch = 0.0f;
  float aElbowYaw = 0.0f;
  float aWristYaw = 0.0f;
  KinematicMatrix hand2Elbow;
  KinematicMatrix hand2HandBase;

  // iterate on circle
  for (int i = 1; i <= circleParts; i++)
  {

    // check if desired p is reachable
    if (pReachable.y() <= robotMetrics_.lengths().maxLElbowY &&
        pReachable.y() >= robotMetrics_.lengths().minLElbowY &&
        pReachable.x() >= getPitchLimit(pReachable.y(), k))
    {
      noAvailableCirclePoint = false;

      aShoulderRoll = std::asin(pReachable.y() / robotMetrics_.link(Links::UPPER_ARM_LENGTH));

      aShoulderPitch = std::atan2(-pReachable.z(), pReachable.x());

      hand2Elbow = KinematicMatrix::transX(-robotMetrics_.link(Links::UPPER_ARM_LENGTH)) *
                   KinematicMatrix::rotZ(-aShoulderRoll) * KinematicMatrix::rotY(-aShoulderPitch) *
                   hand2Shoulder;

      aElbowYaw = std::atan2(-hand2Elbow.posV.z(), -hand2Elbow.posV.y());

      // if ElbowYaw is in range, then optimum is found
      if (aElbowYaw <= robotMetrics_.maxRange(Joints::L_ELBOW_YAW) &&
          aElbowYaw >= robotMetrics_.minRange(Joints::L_ELBOW_YAW))
      {
        optimumFound = true;
        break;
      }

      // store distance to desired hand position
      // set ElbowYaw in Range
      if (aElbowYaw > robotMetrics_.maxRange(Joints::L_ELBOW_YAW))
      {
        aElbowYaw = robotMetrics_.maxRange(Joints::L_ELBOW_YAW);
      }
      else
      {
        aElbowYaw = robotMetrics_.minRange(Joints::L_ELBOW_YAW);
      }

      // transform to handbase space
      hand2HandBase = KinematicMatrix::transX(-robotMetrics_.lengths().foreArmLength) *
                      KinematicMatrix::rotZ(-aElbowRoll) * KinematicMatrix::rotX(-aElbowYaw) *
                      hand2Elbow;

      float dis = hand2HandBase.posV.norm();

      // check if distance to desired hand is better than the best found solution yet
      if (dis < bestDis)
      {
        bestT = t;
        bestDis = dis;
      }
    }

    // step
    t = t + static_cast<float>(i) * step;
    // alternate
    step = -step;
    pReachable = m + u * std::sin(t) + v * std::cos(t);
  }


  // if no optimum could be found
  if (!optimumFound)
  {
    // if there was a possible elbow position on the circle
    if (!noAvailableCirclePoint)
    {
      // take best t
      pReachable = m + u * std::sin(bestT) + v * std::cos(bestT);
    }
    else
    {
      // take the desired elbow position ( not on circle)
      pReachable = pDesired;
    }

    aShoulderRoll = std::asin(pReachable.y() / robotMetrics_.link(Links::UPPER_ARM_LENGTH));

    aShoulderPitch = std::atan2(-pReachable.z(), pReachable.x());

    // check if the angles are in range
    if (aShoulderRoll > robotMetrics_.maxRange(Joints::L_SHOULDER_ROLL))
    {
      aShoulderRoll = robotMetrics_.maxRange(Joints::L_SHOULDER_ROLL);
    }
    else if (aShoulderRoll < robotMetrics_.minRange(Joints::L_SHOULDER_ROLL))
    {
      aShoulderRoll = robotMetrics_.minRange(Joints::L_SHOULDER_ROLL);
    }

    if (aShoulderPitch > robotMetrics_.maxRange(Joints::L_SHOULDER_PITCH))
    {
      aShoulderPitch = robotMetrics_.maxRange(Joints::L_SHOULDER_PITCH);
    }
    else if (aShoulderPitch < robotMetrics_.minRange(Joints::L_SHOULDER_PITCH))
    {
      aShoulderPitch = robotMetrics_.minRange(Joints::L_SHOULDER_PITCH);
    }

    hand2Elbow = KinematicMatrix::transX(-robotMetrics_.link(Links::UPPER_ARM_LENGTH)) *
                 KinematicMatrix::rotZ(-aShoulderRoll) * KinematicMatrix::rotY(-aShoulderPitch) *
                 hand2Shoulder;

    aElbowYaw = std::atan2(-hand2Elbow.posV.z(), -hand2Elbow.posV.y());

    // check elbowYaw
    if (aElbowYaw > robotMetrics_.maxRange(Joints::L_ELBOW_YAW))
    {
      aElbowYaw = robotMetrics_.maxRange(Joints::L_ELBOW_YAW);
    }
    else if (aElbowYaw < robotMetrics_.minRange(Joints::L_ELBOW_YAW))
    {
      aElbowYaw = robotMetrics_.minRange(Joints::L_ELBOW_YAW);
    }
  }
  // transform to handbase space
  hand2HandBase = KinematicMatrix::transX(-robotMetrics_.lengths().foreArmLength) *
                  KinematicMatrix::rotZ(-aElbowRoll) * KinematicMatrix::rotX(-aElbowYaw) *
                  hand2Elbow;

  // calculate WristYaw
  auto hand2handBaseRotM = hand2HandBase.rotM.toRotationMatrix();
  aWristYaw = std::atan2(hand2handBaseRotM(2, 1), hand2handBaseRotM(2, 2));

  if (aWristYaw > robotMetrics_.maxRange(Joints::L_WRIST_YAW))
  {
    aWristYaw = robotMetrics_.maxRange(Joints::L_WRIST_YAW);
  }
  else if (aWristYaw < robotMetrics_.minRange(Joints::L_WRIST_YAW))
  {
    aWristYaw = robotMetrics_.minRange(Joints::L_WRIST_YAW);
  }

  return {{aShoulderPitch, aShoulderRoll, aElbowYaw, aElbowRoll, aWristYaw, handOpening}};
}


JointsArmArray<float> InverseKinematics::getRArmAngles(const KinematicMatrix& desired,
                                                       float handOpening) const
{
  // Transformation of the desired hand position to shoulder space
  KinematicMatrix hand2Shoulder =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::SHOULDER_OFFSET_Z)) *
      KinematicMatrix::transY(robotMetrics_.link(Links::SHOULDER_OFFSET_Y)) * desired;

  // distance from shoulder to desired hand position
  float l = hand2Shoulder.posV.norm();

  // normalized Vector from shoulder to desired hand position
  Vector3f n = hand2Shoulder.posV / l;

  // declare ElbowRoll
  float aElbowRoll = NAN;

  // check, if the desired position is reachable
  if (l > robotMetrics_.lengths().maxArmLength)
  {
    hand2Shoulder.posV = n * robotMetrics_.lengths().maxArmLength;
    l = robotMetrics_.lengths().maxArmLength;
    aElbowRoll = robotMetrics_.minRange(Joints::R_ELBOW_ROLL);
  }
  else if (l < robotMetrics_.lengths().minArmLength)
  {
    hand2Shoulder.posV = n * robotMetrics_.lengths().minArmLength;
    l = robotMetrics_.lengths().minArmLength;
    aElbowRoll = robotMetrics_.maxRange(Joints::R_ELBOW_ROLL);
  }
  else
  {
    // rule of cosines
    aElbowRoll = static_cast<float>(
        -std::acos((std::pow(robotMetrics_.link(Links::UPPER_ARM_LENGTH), 2) +
                    std::pow(robotMetrics_.lengths().foreArmLength, 2) - std::pow(l, 2)) /
                   (2.f * robotMetrics_.link(Links::UPPER_ARM_LENGTH) *
                    robotMetrics_.lengths().foreArmLength)) +
        M_PI);
  }

  // calculation of the circles radius on which the elbow can be positioned
  auto beta = static_cast<float>(
      std::acos((std::pow(l, 2) + std::pow(robotMetrics_.link(Links::UPPER_ARM_LENGTH), 2) -
                 std::pow(robotMetrics_.lengths().foreArmLength, 2)) /
                (2.f * l * robotMetrics_.link(Links::UPPER_ARM_LENGTH))));

  float r = std::sin(beta) * robotMetrics_.link(Links::UPPER_ARM_LENGTH);

  // distance from shoulder to circle midpoint
  float d = std::cos(beta) * robotMetrics_.link(Links::UPPER_ARM_LENGTH);

  // Elbow position from desired hand position and orientation
  KinematicMatrix shoulder2Elbow =
      KinematicMatrix::transX(robotMetrics_.lengths().foreArmLength) * hand2Shoulder.inverted();

  KinematicMatrix elbow2Shoulder = shoulder2Elbow.inverted();

  // distance from desired elbow position to circle surface
  float s = n.dot(elbow2Shoulder.posV) - d;

  // projection of desired elbow position on circle surface
  Vector3f p = elbow2Shoulder.posV - n * s;

  // circle midpoint
  Vector3f m = n * d;

  // Vector from m to p
  Vector3f vecMP = p - m;
  vecMP.normalize();

  // calculate reachable elbow position
  Vector3f pReachable = m + vecMP * r;
  Vector3f pDesired = pReachable;

  /* calculation of rotation angles, such that the shoulder coordinate-system
   * can be transformed to the circle surface.
   * The y- and z- axes are in the surface, the x-axis is the normal vector
   */
  float a1 = std::atan2(m.y(), m.x());
  float a2 = std::atan2(m.z(), std::sqrt(std::pow(m.x(), 2) + std::pow(m.z(), 2)));

  // Transformation matrix to circle space
  KinematicMatrix toCirc = KinematicMatrix::rotZ(a1) * KinematicMatrix::rotY(-a2);

  Vector3f pToCirc = toCirc.inverted() * pReachable;

  float a3 = std::atan2(-pToCirc.y(), pToCirc.z());

  // orthogonal circle vectors
  Vector3f u = toCirc * KinematicMatrix::rotX(a3) * Vector3f(0, r, 0);
  Vector3f v = toCirc * KinematicMatrix::rotX(a3) * Vector3f(0, 0, r);

  // set step size for iteration
  int circleParts = 60;
  float step = 2.f * static_cast<float>(M_PI) / static_cast<float>(circleParts);

  // constant for shoulder roll limits
  float k = std::cos(robotMetrics_.maxRange(Joints::R_SHOULDER_PITCH));

  // iteration variables
  float t = 0;
  float bestDis = std::numeric_limits<float>::infinity();
  float bestT = t;
  bool noAvailableCirclePoint = true;
  bool optimumFound = false;

  float aShoulderRoll = 0.0f;
  float aShoulderPitch = 0.0f;
  float aElbowYaw = 0.0f;
  float aWristYaw = 0.0f;
  KinematicMatrix hand2Elbow;
  KinematicMatrix hand2HandBase;

  // iterate on circle
  for (int i = 1; i <= circleParts; i++)
  {

    // check if desired p is reachable
    if (pReachable.y() <= robotMetrics_.lengths().maxRElbowY &&
        pReachable.y() >= robotMetrics_.lengths().minRElbowY &&
        pReachable.x() >= getPitchLimit(pReachable.y(), k))
    {
      noAvailableCirclePoint = false;

      aShoulderRoll = std::asin(pReachable.y() / robotMetrics_.link(Links::UPPER_ARM_LENGTH));

      aShoulderPitch = std::atan2(-pReachable.z(), pReachable.x());

      hand2Elbow = KinematicMatrix::transX(-robotMetrics_.link(Links::UPPER_ARM_LENGTH)) *
                   KinematicMatrix::rotZ(-aShoulderRoll) * KinematicMatrix::rotY(-aShoulderPitch) *
                   hand2Shoulder;

      aElbowYaw = std::atan2(hand2Elbow.posV.z(), hand2Elbow.posV.y());

      // if ElbowYaw is in range, then optimum is found
      if (aElbowYaw <= robotMetrics_.maxRange(Joints::R_ELBOW_YAW) &&
          aElbowYaw >= robotMetrics_.minRange(Joints::R_ELBOW_YAW))
      {
        optimumFound = true;
        break;
      }

      // store distance to desired hand position
      // set ElbowYaw in Range
      if (aElbowYaw > robotMetrics_.maxRange(Joints::R_ELBOW_YAW))
      {
        aElbowYaw = robotMetrics_.maxRange(Joints::R_ELBOW_YAW);
      }
      else
      {
        aElbowYaw = robotMetrics_.minRange(Joints::R_ELBOW_YAW);
      }

      // transform to handbase space
      hand2HandBase = KinematicMatrix::transX(-robotMetrics_.lengths().foreArmLength) *
                      KinematicMatrix::rotZ(-aElbowRoll) * KinematicMatrix::rotX(-aElbowYaw) *
                      hand2Elbow;

      float dis = hand2HandBase.posV.norm();

      // check if distance to desired hand is better than the best found solution yet
      if (dis < bestDis)
      {
        bestT = t;
        bestDis = dis;
      }
    }

    // step
    t = t + static_cast<float>(i) * step;
    // alternate
    step = -step;
    pReachable = m + u * std::sin(t) + v * std::cos(t);
  }


  // if no optimum could be found
  if (!optimumFound)
  {
    // if there was a possible elbow position on the circle
    if (!noAvailableCirclePoint)
    {
      // take best t
      pReachable = m + u * std::sin(bestT) + v * std::cos(bestT);
    }
    else
    {
      // take the desired elbow position ( not on circle)
      pReachable = pDesired;
    }

    aShoulderRoll = std::asin(pReachable.y() / robotMetrics_.link(Links::UPPER_ARM_LENGTH));

    aShoulderPitch = std::atan2(-pReachable.z(), pReachable.x());

    // check if the angles are in range
    if (aShoulderRoll > robotMetrics_.maxRange(Joints::R_SHOULDER_ROLL))
    {
      aShoulderRoll = robotMetrics_.maxRange(Joints::R_SHOULDER_ROLL);
    }
    else if (aShoulderRoll < robotMetrics_.minRange(Joints::R_SHOULDER_ROLL))
    {
      aShoulderRoll = robotMetrics_.minRange(Joints::R_SHOULDER_ROLL);
    }

    if (aShoulderPitch > robotMetrics_.maxRange(Joints::R_SHOULDER_PITCH))
    {
      aShoulderPitch = robotMetrics_.maxRange(Joints::R_SHOULDER_PITCH);
    }
    else if (aShoulderPitch < robotMetrics_.minRange(Joints::R_SHOULDER_PITCH))
    {
      aShoulderPitch = robotMetrics_.minRange(Joints::R_SHOULDER_PITCH);
    }

    hand2Elbow = KinematicMatrix::transX(-robotMetrics_.link(Links::UPPER_ARM_LENGTH)) *
                 KinematicMatrix::rotZ(-aShoulderRoll) * KinematicMatrix::rotY(-aShoulderPitch) *
                 hand2Shoulder;

    aElbowYaw = std::atan2(hand2Elbow.posV.z(), hand2Elbow.posV.y());

    // check elbowYaw
    if (aElbowYaw > robotMetrics_.maxRange(Joints::R_ELBOW_YAW))
    {
      aElbowYaw = robotMetrics_.maxRange(Joints::R_ELBOW_YAW);
    }
    else if (aElbowYaw < robotMetrics_.minRange(Joints::R_ELBOW_YAW))
    {
      aElbowYaw = robotMetrics_.minRange(Joints::R_ELBOW_YAW);
    }
  }

  // transform to handbase space
  hand2HandBase = KinematicMatrix::transX(-robotMetrics_.lengths().foreArmLength) *
                  KinematicMatrix::rotZ(-aElbowRoll) * KinematicMatrix::rotX(-aElbowYaw) *
                  hand2Elbow;

  // calculate WristYaw
  auto hand2handBaseRotM = hand2HandBase.rotM.toRotationMatrix();
  aWristYaw = std::atan2(hand2handBaseRotM(2, 1), hand2handBaseRotM(2, 2));

  if (aWristYaw > robotMetrics_.maxRange(Joints::R_WRIST_YAW))
  {
    aWristYaw = robotMetrics_.maxRange(Joints::R_WRIST_YAW);
  }
  else if (aWristYaw < robotMetrics_.minRange(Joints::R_WRIST_YAW))
  {
    aWristYaw = robotMetrics_.minRange(Joints::R_WRIST_YAW);
  }

  return {{aShoulderPitch, aShoulderRoll, aElbowYaw, aElbowRoll, aWristYaw, handOpening}};
}


JointsLegArray<float> InverseKinematics::getFixedLLegAngles(const KinematicMatrix& desired,
                                                            float aHipYawPitch) const
{
  // store hipyawpitch
  float hyp = aHipYawPitch;

  // check if given HipYawPitch is in range
  if (hyp > robotMetrics_.maxRange(Joints::L_HIP_YAW_PITCH))
  {
    hyp = robotMetrics_.maxRange(Joints::L_HIP_YAW_PITCH);
  }
  else if (hyp < robotMetrics_.minRange(Joints::L_HIP_YAW_PITCH))
  {
    hyp = robotMetrics_.minRange(Joints::L_HIP_YAW_PITCH);
  }


  // First we need the torso position from foot space to calculate the desired angle position
  KinematicMatrix torso2ankle =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::FOOT_HEIGHT)) * desired.inverted();

  // Invert to get the desired annkle position
  KinematicMatrix ankleDesired = torso2ankle.inverted();

  // transformation of the desired ankle position to rotated hip space
  KinematicMatrix ankle2hipOrthogonal =
      KinematicMatrix::rotX(-45.0f * TO_RAD) *
      KinematicMatrix::transY(-robotMetrics_.link(Links::HIP_OFFSET_Y)) *
      KinematicMatrix::transZ(robotMetrics_.link(Links::HIP_OFFSET_Z)) * ankleDesired;

  // transformation to space rotated about fixed HipYawPitch angle
  KinematicMatrix ankle2RotatedHipOrthogonal = KinematicMatrix::rotZ(hyp) * ankle2hipOrthogonal;

  // distance from ankle to hip
  float l = ankle2RotatedHipOrthogonal.posV.norm();

  // normal vector to ankle
  Vector3f n = ankle2RotatedHipOrthogonal.posV / l;

  float aKneePitch = NAN;

  // reachability check
  if (l > robotMetrics_.lengths().maxLegLength)
  {
    ankle2RotatedHipOrthogonal.posV = n * robotMetrics_.lengths().maxLegLength;
    l = robotMetrics_.lengths().maxLegLength;
    aKneePitch = 0.0f;
  }
  else if (l < robotMetrics_.lengths().minLegLength)
  {
    ankle2RotatedHipOrthogonal.posV = n * robotMetrics_.lengths().minLegLength;
    l = robotMetrics_.lengths().minLegLength;
    aKneePitch = robotMetrics_.maxRange(Joints::L_KNEE_PITCH);
  }
  else
  {
    // calculation of kneePitch with rule of cosines
    aKneePitch = static_cast<float>(
        M_PI - std::acos((std::pow(robotMetrics_.link(Links::THIGH_LENGTH), 2) +
                          std::pow(robotMetrics_.link(Links::TIBIA_LENGTH), 2) - std::pow(l, 2)) /
                         (2.f * robotMetrics_.link(Links::THIGH_LENGTH) *
                          robotMetrics_.link(Links::TIBIA_LENGTH))));
  }

  // calculation of HipPitch from triangle and position of ankle
  auto aHipPitch = static_cast<float>(
      -(std::acos((pow(robotMetrics_.link(Links::THIGH_LENGTH), 2) -
                   std::pow(robotMetrics_.link(Links::TIBIA_LENGTH), 2) + std::pow(l, 2)) /
                  (2 * robotMetrics_.link(Links::THIGH_LENGTH) * l)) +
        std::asin(ankle2RotatedHipOrthogonal.posV.x() / l)));

  // calculation of hip roll angle from position of ankle
  float aHipRoll =
      std::atan2(ankle2RotatedHipOrthogonal.posV.z(), ankle2RotatedHipOrthogonal.posV.y()) +
      3.0f / 4.0f * static_cast<float>(M_PI);

  // hold hip angles in range
  if (aHipPitch > robotMetrics_.maxRange(Joints::L_HIP_PITCH))
  {
    aHipPitch = robotMetrics_.maxRange(Joints::L_HIP_PITCH);
  }
  else if (aHipPitch < robotMetrics_.minRange(Joints::L_HIP_PITCH))
  {
    aHipPitch = robotMetrics_.minRange(Joints::L_HIP_PITCH);
  }

  if (aHipRoll > robotMetrics_.maxRange(Joints::L_HIP_ROLL))
  {
    aHipRoll = robotMetrics_.maxRange(Joints::L_HIP_ROLL);
  }
  else if (aHipRoll < robotMetrics_.minRange(Joints::L_HIP_ROLL))
  {
    aHipRoll = robotMetrics_.minRange(Joints::L_HIP_ROLL);
  }

  // transformation to ankle space
  KinematicMatrix ankleRotated2ankle =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::TIBIA_LENGTH)) *
      KinematicMatrix::rotY(aKneePitch) *
      KinematicMatrix::transZ(-robotMetrics_.link(Links::THIGH_LENGTH)) *
      KinematicMatrix::rotY(aHipPitch) *
      KinematicMatrix::rotX(-(aHipRoll + 3.0f / 4.0f * static_cast<float>(M_PI))) *
      ankle2RotatedHipOrthogonal;

  auto ankleRotationMatrix = ankleRotated2ankle.rotM.toRotationMatrix();
  float aAnkleRoll = std::asin(ankleRotationMatrix(1, 2));
  float aAnklePitch = -(std::atan2(-ankleRotationMatrix(0, 2), -ankleRotationMatrix(2, 2)));

  // hold ankle angles in range
  if (aAnklePitch > robotMetrics_.maxRange(Joints::L_ANKLE_PITCH))
  {
    aAnklePitch = robotMetrics_.maxRange(Joints::L_ANKLE_PITCH);
  }
  else if (aAnklePitch < robotMetrics_.minRange(Joints::L_ANKLE_PITCH))
  {
    aAnklePitch = robotMetrics_.minRange(Joints::L_ANKLE_PITCH);
  }

  if (aAnkleRoll > robotMetrics_.maxRangeLAnkleRoll(aAnklePitch))
  {
    aAnkleRoll = robotMetrics_.maxRangeLAnkleRoll(aAnklePitch);
  }
  else if (aAnkleRoll < robotMetrics_.minRangeLAnkleRoll(aAnklePitch))
  {
    aAnkleRoll = robotMetrics_.minRangeLAnkleRoll(aAnklePitch);
  }


  return {{hyp, aHipRoll, aHipPitch, aKneePitch, aAnklePitch, aAnkleRoll}};
}

JointsLegArray<float> InverseKinematics::getFixedRLegAngles(const KinematicMatrix& desired,
                                                            float aHipYawPitch) const
{

  // store hipyawpitch
  float hyp = aHipYawPitch;

  // check if given HipYawPitch is in range
  if (hyp > robotMetrics_.maxRange(Joints::R_HIP_YAW_PITCH))
  {
    hyp = robotMetrics_.maxRange(Joints::R_HIP_YAW_PITCH);
  }
  else if (hyp < robotMetrics_.minRange(Joints::R_HIP_YAW_PITCH))
  {
    hyp = robotMetrics_.minRange(Joints::R_HIP_YAW_PITCH);
  }


  // First we need the torso position from foot space to calculate the desired angle position
  KinematicMatrix torso2ankle =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::FOOT_HEIGHT)) * desired.inverted();

  // Invert to get the desired annkle position
  KinematicMatrix ankleDesired = torso2ankle.inverted();

  // transformation of the desired ankle position to rotated hip space
  KinematicMatrix ankle2hipOrthogonal =
      KinematicMatrix::rotX(45.0f * TO_RAD) *
      KinematicMatrix::transY(robotMetrics_.link(Links::HIP_OFFSET_Y)) *
      KinematicMatrix::transZ(robotMetrics_.link(Links::HIP_OFFSET_Z)) * ankleDesired;

  // transformation to space rotated about fixed HipYawPitch angle
  KinematicMatrix ankle2RotatedHipOrthogonal = KinematicMatrix::rotZ(-hyp) * ankle2hipOrthogonal;

  // distance from ankle to hip
  float l = ankle2RotatedHipOrthogonal.posV.norm();

  // normal vector to ankle
  Vector3f n = ankle2RotatedHipOrthogonal.posV / l;

  float aKneePitch = NAN;

  // reachability check
  if (l > robotMetrics_.lengths().maxLegLength)
  {
    ankle2RotatedHipOrthogonal.posV = n * robotMetrics_.lengths().maxLegLength;
    l = robotMetrics_.lengths().maxLegLength;
    aKneePitch = 0.0f;
  }
  else if (l < robotMetrics_.lengths().minLegLength)
  {
    ankle2RotatedHipOrthogonal.posV = n * robotMetrics_.lengths().minLegLength;
    l = robotMetrics_.lengths().minLegLength;
    aKneePitch = robotMetrics_.maxRange(Joints::R_KNEE_PITCH);
  }
  else
  {
    // calculation of kneePitch with rule of cosines
    aKneePitch = static_cast<float>(
        M_PI - std::acos((std::pow(robotMetrics_.link(Links::THIGH_LENGTH), 2) +
                          std::pow(robotMetrics_.link(Links::TIBIA_LENGTH), 2) - std::pow(l, 2)) /
                         (2.f * robotMetrics_.link(Links::THIGH_LENGTH) *
                          robotMetrics_.link(Links::TIBIA_LENGTH))));
  }

  // calculation of HipPitch from triangle and position of ankle
  auto aHipPitch = static_cast<float>(
      -(std::acos((std::pow(robotMetrics_.link(Links::THIGH_LENGTH), 2) -
                   std::pow(robotMetrics_.link(Links::TIBIA_LENGTH), 2) + std::pow(l, 2)) /
                  (2 * robotMetrics_.link(Links::THIGH_LENGTH) * l)) +
        std::asin(ankle2RotatedHipOrthogonal.posV.x() / l)));

  // calculation of hip roll angle from position of ankle
  float aHipRoll =
      std::atan2(ankle2RotatedHipOrthogonal.posV.z(), ankle2RotatedHipOrthogonal.posV.y()) +
      1.0f / 4.0f * static_cast<float>(M_PI);

  // hold hip angles in range
  if (aHipPitch > robotMetrics_.maxRange(Joints::R_HIP_PITCH))
  {
    aHipPitch = robotMetrics_.maxRange(Joints::R_HIP_PITCH);
  }
  else if (aHipPitch < robotMetrics_.minRange(Joints::R_HIP_PITCH))
  {
    aHipPitch = robotMetrics_.minRange(Joints::R_HIP_PITCH);
  }

  if (aHipRoll > robotMetrics_.maxRange(Joints::R_HIP_ROLL))
  {
    aHipRoll = robotMetrics_.maxRange(Joints::R_HIP_ROLL);
  }
  else if (aHipRoll < robotMetrics_.minRange(Joints::R_HIP_ROLL))
  {
    aHipRoll = robotMetrics_.minRange(Joints::R_HIP_ROLL);
  }

  // transformation to ankle space
  KinematicMatrix ankleRotated2ankle =
      KinematicMatrix::transZ(robotMetrics_.link(Links::TIBIA_LENGTH)) *
      KinematicMatrix::rotY(-aKneePitch) *
      KinematicMatrix::transZ(robotMetrics_.link(Links::THIGH_LENGTH)) *
      KinematicMatrix::rotY(-aHipPitch) *
      KinematicMatrix::rotX(-(aHipRoll + 1.0f / 4.0f * static_cast<float>(M_PI))) *
      ankle2RotatedHipOrthogonal;

  auto ankleRotationMatrix = ankleRotated2ankle.rotM.toRotationMatrix();
  float aAnkleRoll = -std::asin(ankleRotationMatrix(1, 2));
  float aAnklePitch = -(std::atan2(-ankleRotationMatrix(0, 2), ankleRotationMatrix(2, 2)));

  // hold ankle angles in range
  if (aAnklePitch > robotMetrics_.maxRange(Joints::R_ANKLE_PITCH))
  {
    aAnklePitch = robotMetrics_.maxRange(Joints::R_ANKLE_PITCH);
  }
  else if (aAnklePitch < robotMetrics_.minRange(Joints::R_ANKLE_PITCH))
  {
    aAnklePitch = robotMetrics_.minRange(Joints::R_ANKLE_PITCH);
  }

  if (aAnkleRoll > robotMetrics_.maxRangeRAnkleRoll(aAnklePitch))
  {
    aAnkleRoll = robotMetrics_.maxRangeRAnkleRoll(aAnklePitch);
  }
  else if (aAnkleRoll < robotMetrics_.minRangeRAnkleRoll(aAnklePitch))
  {
    aAnkleRoll = robotMetrics_.minRangeRAnkleRoll(aAnklePitch);
  }


  return {{hyp, aHipRoll, aHipPitch, aKneePitch, aAnklePitch, aAnkleRoll}};
}
