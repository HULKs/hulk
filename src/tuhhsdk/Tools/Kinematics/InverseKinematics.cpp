#include "InverseKinematics.h"
#include <Modules/NaoProvider.h>

using namespace JOINTS;
using namespace LINKS;
using namespace std;

vector<float> InverseKinematics::getLLegAngles(const KinematicMatrix& desired)
{
  // given is the desired position and orientation of the foot
  // but we need the desired position and rotation of the ankle
  // first transform to ankle space and shift about FOOT_HEIGTH
  // to get the desired ankle
  KinematicMatrix ankleInv = KinematicMatrix::transZ(-NaoProvider::link(FOOT_HEIGHT)) * desired.invert();

  // now transform back to torso space
  KinematicMatrix ankleDesired = ankleInv.invert();

  // transformation of the desired position to the Hip Space
  KinematicMatrix ankle2hip =
      KinematicMatrix::transY(-NaoProvider::link(HIP_OFFSET_Y)) * KinematicMatrix::transZ(NaoProvider::link(HIP_OFFSET_Z)) * ankleDesired;

  // Transformation to the rotated Hip Space
  KinematicMatrix ankle2hipOrthogonal = KinematicMatrix::rotX(-45.0f * TO_RAD) * ankle2hip;

  // calculate the the distance from hip to ankle
  float l = ankle2hipOrthogonal.posV.norm();
  // normal vektor to ankle
  Vector3f n = ankle2hipOrthogonal.posV / l;

  // check wether the position is reachable
  float a_KneePitch;

  if (l > NaoProvider::maxLegLength())
  {
    ankle2hipOrthogonal.posV = n * NaoProvider::maxLegLength();
    l = NaoProvider::maxLegLength();
    a_KneePitch = 0.0f;
  }
  else if (l < NaoProvider::minLegLength())
  {
    ankle2hipOrthogonal.posV = n * NaoProvider::minLegLength();
    l = NaoProvider::minLegLength();
    a_KneePitch = NaoProvider::maxRange(L_KNEE_PITCH);
  }
  else
  {
    // calculate the knee angle from thigh length, tibia length and hip-ankle distance
    a_KneePitch = (float)M_PI - acos((pow(NaoProvider::link(THIGH_LENGTH), 2) + pow(NaoProvider::link(TIBIA_LENGTH), 2) - pow(l, 2)) /
                                     (2 * NaoProvider::link(THIGH_LENGTH) * NaoProvider::link(TIBIA_LENGTH)));
  }

  // inverse needed
  KinematicMatrix hipOrthogonal2ankle = ankle2hipOrthogonal.invert();

  // calculate angle for ankle pitch
  float a_AnklePitch_1 =
      acos((pow(NaoProvider::link(TIBIA_LENGTH), 2) + pow(l, 2) - pow(NaoProvider::link(THIGH_LENGTH), 2)) / (2 * NaoProvider::link(TIBIA_LENGTH) * l));

  Vector3f v_hipAnkle = hipOrthogonal2ankle.posV;
  float a_AnklePitch2 = atan2(v_hipAnkle.x(), sqrt(pow(v_hipAnkle.y(), 2) + pow(v_hipAnkle.z(), 2)));

  float a_AnklePitch = -(a_AnklePitch_1 + a_AnklePitch2);

  // calculate angle for ankle roll
  float a_AnkleRoll = atan2(v_hipAnkle.y(), v_hipAnkle.z());


  // transform the desired position from ankle space to hip
  KinematicMatrix thigh2Foot = KinematicMatrix::rotX(-a_AnkleRoll) * KinematicMatrix::rotY(-a_AnklePitch) *
                               KinematicMatrix::transZ(NaoProvider::link(TIBIA_LENGTH)) * KinematicMatrix::rotY(-a_KneePitch) *
                               KinematicMatrix::transZ(NaoProvider::link(THIGH_LENGTH));

  // get the transformation to Hip Orthogonal
  KinematicMatrix hipOrthogonal2thigh = ankle2hipOrthogonal * thigh2Foot;

  // get angles from the transformation matrix
  auto hipRotM = hipOrthogonal2thigh.rotM.toRotationMatrix();
  float alphaX = asin(hipRotM(2, 1));
  float a_HipYawPitch = -atan2(-hipRotM(0, 1), hipRotM(1, 1));
  float a_HipPitch = atan2(-hipRotM(2, 0), hipRotM(2, 2));
  float a_HipRoll = (float)(alphaX + M_PI / 4);

  // constraints on angles

  // ankle Pitch
  if (a_AnklePitch < NaoProvider::minRange(L_ANKLE_PITCH))
    a_AnklePitch = NaoProvider::minRange(L_ANKLE_PITCH);
  else if (a_AnklePitch > NaoProvider::maxRange(L_ANKLE_PITCH))
    a_AnklePitch = NaoProvider::maxRange(L_ANKLE_PITCH);

  // ankleRoll
  if (a_AnkleRoll < NaoProvider::minRangeLAnkleRoll(a_AnklePitch))
    a_AnkleRoll = NaoProvider::minRangeLAnkleRoll(a_AnklePitch);
  else if (a_AnkleRoll > NaoProvider::maxRangeLAnkleRoll(a_AnklePitch))
    a_AnkleRoll = NaoProvider::maxRangeLAnkleRoll(a_AnklePitch);

  // hipYaw
  if (a_HipYawPitch < NaoProvider::minRange(L_HIP_YAW_PITCH))
    a_HipYawPitch = NaoProvider::minRange(L_HIP_YAW_PITCH);
  else if (a_HipYawPitch > NaoProvider::maxRange(L_HIP_YAW_PITCH))
    a_HipYawPitch = NaoProvider::maxRange(L_HIP_YAW_PITCH);

  // hipPitch
  if (a_HipPitch < NaoProvider::minRange(L_HIP_PITCH))
    a_HipPitch = NaoProvider::minRange(L_HIP_PITCH);
  else if (a_HipPitch > NaoProvider::maxRange(L_HIP_PITCH))
    a_HipPitch = NaoProvider::maxRange(L_HIP_PITCH);

  // hipRoll
  if (a_HipRoll < NaoProvider::minRange(L_HIP_ROLL))
    a_HipRoll = NaoProvider::minRange(L_HIP_ROLL);
  else if (a_HipRoll > NaoProvider::maxRange(L_HIP_ROLL))
    a_HipRoll = NaoProvider::maxRange(L_HIP_ROLL);

  // create angle vector
  vector<float> leg;
  leg.push_back(a_HipYawPitch);
  leg.push_back(a_HipRoll);
  leg.push_back(a_HipPitch);
  leg.push_back(a_KneePitch);
  leg.push_back(a_AnklePitch);
  leg.push_back(a_AnkleRoll);

  return leg;
}


vector<float> InverseKinematics::getRLegAngles(const KinematicMatrix& desired)
{

  // given is the desired position and orientation of the foot
  // but we need the desired position and rotation of the ankle
  // first transform to ankle space and shift about FOOT_HEIGTH
  // to get the desired ankle
  KinematicMatrix ankleInv = KinematicMatrix::transZ(-NaoProvider::link(FOOT_HEIGHT)) * desired.invert();

  // transform back to torso space
  KinematicMatrix ankleDesired = ankleInv.invert();

  // transformation of the desired position to the Hip Space
  KinematicMatrix ankle2hip =
      KinematicMatrix::transY(NaoProvider::link(HIP_OFFSET_Y)) * KinematicMatrix::transZ(NaoProvider::link(HIP_OFFSET_Z)) * ankleDesired;

  // Transformation to the rotated Hip Space
  KinematicMatrix ankle2hipOrthogonal = KinematicMatrix::rotX(45.0f * TO_RAD) * ankle2hip;

  // calculate the the distance from hip to ankle
  float l = ankle2hipOrthogonal.posV.norm();

  // normal vector to ankle
  Vector3f n = ankle2hipOrthogonal.posV / l;

  // check wether the position is reachable
  float a_KneePitch;

  if (l > NaoProvider::maxLegLength())
  {
    ankle2hipOrthogonal.posV = n * NaoProvider::maxLegLength();
    l = NaoProvider::maxLegLength();
    a_KneePitch = 0.0f;
  }
  else if (l < NaoProvider::minLegLength())
  {
    ankle2hipOrthogonal.posV = n * NaoProvider::minLegLength();
    l = NaoProvider::minLegLength();
    a_KneePitch = NaoProvider::maxRange(R_KNEE_PITCH);
  }
  else
  {
    // calculate the knee angle from thigh length, tibia length and hip-ankle distance
    a_KneePitch = (float)M_PI - acos((pow(NaoProvider::link(THIGH_LENGTH), 2) + pow(NaoProvider::link(TIBIA_LENGTH), 2) - pow(l, 2)) /
                                     (2 * NaoProvider::link(THIGH_LENGTH) * NaoProvider::link(TIBIA_LENGTH)));
  }

  // inverse needed
  KinematicMatrix hipOrthogonal2ankle = ankle2hipOrthogonal.invert();

  // calculate angle for ankle pitch
  float a_AnklePitch_1 =
      acos((pow(NaoProvider::link(TIBIA_LENGTH), 2) + pow(l, 2) - pow(NaoProvider::link(THIGH_LENGTH), 2)) / (2 * NaoProvider::link(TIBIA_LENGTH) * l));

  Vector3f v_hipAnkle = hipOrthogonal2ankle.posV;
  float a_AnklePitch2 = atan2(v_hipAnkle.x(), sqrt(pow(v_hipAnkle.y(), 2) + pow(v_hipAnkle.z(), 2)));

  float a_AnklePitch = -(a_AnklePitch_1 + a_AnklePitch2);

  // calculate angle for ankle roll
  float a_AnkleRoll = atan2(v_hipAnkle.y(), v_hipAnkle.z());


  // transform the desired position from ankle space to hip
  KinematicMatrix thigh2Foot = KinematicMatrix::rotX(-a_AnkleRoll) * KinematicMatrix::rotY(-a_AnklePitch) *
                               KinematicMatrix::transZ(NaoProvider::link(TIBIA_LENGTH)) * KinematicMatrix::rotY(-a_KneePitch) *
                               KinematicMatrix::transZ(NaoProvider::link(THIGH_LENGTH));

  // get the transformation to Hip Orthogonal
  KinematicMatrix hipOrthogonal2thigh = ankle2hipOrthogonal * thigh2Foot;

  // get angles from the transformation matrix
  auto hipRotM = hipOrthogonal2thigh.rotM.toRotationMatrix();
  float alphaX = asin(hipRotM(2, 1));
  float a_HipYawPitch = atan2(-hipRotM(0, 1), hipRotM(1, 1));
  float a_HipPitch = atan2(-hipRotM(2, 0), hipRotM(2, 2));
  float a_HipRoll = alphaX - (float)M_PI / 4;

  // constraints on angles

  // ankle Pitch
  if (a_AnklePitch < NaoProvider::minRange(R_ANKLE_PITCH))
    a_AnklePitch = NaoProvider::minRange(R_ANKLE_PITCH);
  else if (a_AnklePitch > NaoProvider::maxRange(R_ANKLE_PITCH))
    a_AnklePitch = NaoProvider::maxRange(R_ANKLE_PITCH);

  // ankleRoll
  if (a_AnkleRoll < NaoProvider::minRangeRAnkleRoll(a_AnklePitch))
    a_AnkleRoll = NaoProvider::minRangeRAnkleRoll(a_AnklePitch);
  else if (a_AnkleRoll > NaoProvider::maxRangeRAnkleRoll(a_AnklePitch))
    a_AnkleRoll = NaoProvider::maxRangeRAnkleRoll(a_AnklePitch);

  // hipYaw
  if (a_HipYawPitch < NaoProvider::minRange(R_HIP_YAW_PITCH))
    a_HipYawPitch = NaoProvider::minRange(R_HIP_YAW_PITCH);
  else if (a_HipYawPitch > NaoProvider::maxRange(R_HIP_YAW_PITCH))
    a_HipYawPitch = NaoProvider::maxRange(R_HIP_YAW_PITCH);

  // hipPitch
  if (a_HipPitch < NaoProvider::minRange(R_HIP_PITCH))
    a_HipPitch = NaoProvider::minRange(R_HIP_PITCH);
  else if (a_HipPitch > NaoProvider::maxRange(R_HIP_PITCH))
    a_HipPitch = NaoProvider::maxRange(R_HIP_PITCH);

  // hipRoll
  if (a_HipRoll < NaoProvider::minRange(R_HIP_ROLL))
    a_HipRoll = NaoProvider::minRange(R_HIP_ROLL);
  else if (a_HipRoll > NaoProvider::maxRange(R_HIP_ROLL))
    a_HipRoll = NaoProvider::maxRange(R_HIP_ROLL);

  // create angle vector
  vector<float> leg;
  leg.push_back(a_HipYawPitch);
  leg.push_back(a_HipRoll);
  leg.push_back(a_HipPitch);
  leg.push_back(a_KneePitch);
  leg.push_back(a_AnklePitch);
  leg.push_back(a_AnkleRoll);

  return leg;
}


vector<float> InverseKinematics::getLArmAngles(const KinematicMatrix& desired, const float& handOpening)
{

  // Transformation of the desired hand position to shoulder space
  KinematicMatrix Hand2Shoulder =
      KinematicMatrix::transZ(-NaoProvider::link(SHOULDER_OFFSET_Z)) * KinematicMatrix::transY(-NaoProvider::link(SHOULDER_OFFSET_Y)) * desired;

  // distance from shoulder to desired hand position
  float l = Hand2Shoulder.posV.norm();

  // normalized Vector from shoulder to desired hand position
  Vector3f n = Hand2Shoulder.posV / l;

  // declar ElbowRoll
  float a_ElbowRoll;

  // check, if the desired position is reachable
  if (l > NaoProvider::maxArmLength())
  {
    Hand2Shoulder.posV = n * NaoProvider::maxArmLength();
    l = NaoProvider::maxArmLength();
    a_ElbowRoll = NaoProvider::maxRange(L_ELBOW_ROLL);
  }
  else if (l < NaoProvider::minArmLength())
  {
    Hand2Shoulder.posV = n * NaoProvider::minArmLength();
    l = NaoProvider::minArmLength();
    a_ElbowRoll = NaoProvider::minRange(L_ELBOW_ROLL);
  }
  else
  {
    // rule of cosines
    a_ElbowRoll = acos((pow(NaoProvider::link(UPPER_ARM_LENGTH), 2) + pow(NaoProvider::foreArmLength(), 2) - pow(l, 2)) /
                       (2 * NaoProvider::link(UPPER_ARM_LENGTH) * NaoProvider::foreArmLength())) -
                  (float)M_PI;
  }

  // calculation of the circles radius on which the elbow can be positioned
  float beta =
      acos((pow(l, 2) + pow(NaoProvider::link(UPPER_ARM_LENGTH), 2) - pow(NaoProvider::foreArmLength(), 2)) / (2 * l * NaoProvider::link(UPPER_ARM_LENGTH)));

  float r = sin(beta) * NaoProvider::link(UPPER_ARM_LENGTH);

  // distance from shoulder to circle midpoint
  float d = cos(beta) * NaoProvider::link(UPPER_ARM_LENGTH);

  // Elbow position from desired hand position and orientation
  KinematicMatrix Shoulder2Elbow = KinematicMatrix::transX(NaoProvider::foreArmLength()) * Hand2Shoulder.invert();

  KinematicMatrix Elbow2Shoulder = Shoulder2Elbow.invert();

  // distance from desired elbow position to circle surface
  float s = n.dot(Elbow2Shoulder.posV) - d;

  // projection of desired elbow position on circle surface
  Vector3f p = Elbow2Shoulder.posV - n * s;

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
  float a1 = atan2(m.y(), m.x());
  float a2 = atan2(m.z(), sqrtf(pow(m.x(), 2) + pow(m.z(), 2)));

  // Transformation matrix to circle space
  KinematicMatrix ToCirc = KinematicMatrix::rotZ(a1) * KinematicMatrix::rotY(-a2);

  Vector3f pToCirc = ToCirc.invert() * pReachable;

  float a3 = atan2(-pToCirc.y(), pToCirc.z());

  // orthogonal circle vectors
  Vector3f u = ToCirc * KinematicMatrix::rotX(a3) * Vector3f(0, r, 0);
  Vector3f v = ToCirc * KinematicMatrix::rotX(a3) * Vector3f(0, 0, r);

  // set step size for iteration
  int circleParts = 60;
  float step = 2 * (float)M_PI / circleParts;

  // constant for shoulder roll limits
  float k = cos(NaoProvider::maxRange(L_SHOULDER_PITCH));

  // iteration variables
  float t = 0;
  float bestDis = std::numeric_limits<float>::infinity();
  float bestT = t;
  bool noAvailableCirclePoint = true;
  bool optimumFound = false;

  float a_ShoulderRoll = 0.0f;
  float a_ShoulderPitch = 0.0f;
  float a_ElbowYaw = 0.0f;
  float a_WristYaw = 0.0f;
  KinematicMatrix Hand2Elbow;
  KinematicMatrix Hand2HandBase;

  // iterate on circle
  for (int i = 1; i <= circleParts; i++)
  {

    // check if desired p is reachable
    if (pReachable.y() <= NaoProvider::maxLElbowY() && pReachable.y() >= NaoProvider::minLElbowY() && pReachable.x() >= getPitchlimit(pReachable.y(), k))
    {
      noAvailableCirclePoint = false;

      a_ShoulderRoll = asin(pReachable.y() / NaoProvider::link(UPPER_ARM_LENGTH));

      a_ShoulderPitch = atan2(-pReachable.z(), pReachable.x());

      Hand2Elbow = KinematicMatrix::transX(-NaoProvider::link(UPPER_ARM_LENGTH)) * KinematicMatrix::rotZ(-a_ShoulderRoll) *
                   KinematicMatrix::rotY(-a_ShoulderPitch) * Hand2Shoulder;

      a_ElbowYaw = atan2(-Hand2Elbow.posV.z(), -Hand2Elbow.posV.y());

      // if ElbowYaw is in range, then optimum is found
      if (a_ElbowYaw <= NaoProvider::maxRange(L_ELBOW_YAW) && a_ElbowYaw >= NaoProvider::minRange(L_ELBOW_YAW))
      {
        optimumFound = true;
        break;
      }
      else
      {
        // store distance to desired hand position
        // set ElbowYaw in Range
        if (a_ElbowYaw > NaoProvider::maxRange(L_ELBOW_YAW))
          a_ElbowYaw = NaoProvider::maxRange(L_ELBOW_YAW);
        else
          a_ElbowYaw = NaoProvider::minRange(L_ELBOW_YAW);

        // transform to handbase space
        Hand2HandBase =
            KinematicMatrix::transX(-NaoProvider::foreArmLength()) * KinematicMatrix::rotZ(-a_ElbowRoll) * KinematicMatrix::rotX(-a_ElbowYaw) * Hand2Elbow;

        float dis = Hand2HandBase.posV.norm();

        // check if distance to desired hand is better than the best found solution yet
        if (dis < bestDis)
        {
          bestT = t;
          bestDis = dis;
        }
      }
    }

    // step
    t = t + i * step;
    // alternate
    step = -step;
    pReachable = m + u * sin(t) + v * cos(t);
  }


  // if no optimum could be found
  if (!optimumFound)
  {
    // if there was a possible elbow position on the circle
    if (!noAvailableCirclePoint)
      // take best t
      pReachable = m + u * sin(bestT) + v * cos(bestT);
    else
      // take the desired elbow position ( not on circle)
      pReachable = pDesired;

    a_ShoulderRoll = asin(pReachable.y() / NaoProvider::link(UPPER_ARM_LENGTH));

    a_ShoulderPitch = atan2(-pReachable.z(), pReachable.x());

    // check if the angles are in range
    if (a_ShoulderRoll > NaoProvider::maxRange(L_SHOULDER_ROLL))
      a_ShoulderRoll = NaoProvider::maxRange(L_SHOULDER_ROLL);
    else if (a_ShoulderRoll < NaoProvider::minRange(L_SHOULDER_ROLL))
      a_ShoulderRoll = NaoProvider::minRange(L_SHOULDER_ROLL);

    if (a_ShoulderPitch > NaoProvider::maxRange(L_SHOULDER_PITCH))
      a_ShoulderPitch = NaoProvider::maxRange(L_SHOULDER_PITCH);
    else if (a_ShoulderPitch < NaoProvider::minRange(L_SHOULDER_PITCH))
      a_ShoulderPitch = NaoProvider::minRange(L_SHOULDER_PITCH);

    Hand2Elbow = KinematicMatrix::transX(-NaoProvider::link(UPPER_ARM_LENGTH)) * KinematicMatrix::rotZ(-a_ShoulderRoll) *
                 KinematicMatrix::rotY(-a_ShoulderPitch) * Hand2Shoulder;

    a_ElbowYaw = atan2(-Hand2Elbow.posV.z(), -Hand2Elbow.posV.y());

    // check elbowYaw
    if (a_ElbowYaw > NaoProvider::maxRange(L_ELBOW_YAW))
      a_ElbowYaw = NaoProvider::maxRange(L_ELBOW_YAW);
    else if (a_ElbowYaw < NaoProvider::minRange(L_ELBOW_YAW))
      a_ElbowYaw = NaoProvider::minRange(L_ELBOW_YAW);
  }
  // transform to handbase space
  Hand2HandBase =
      KinematicMatrix::transX(-NaoProvider::foreArmLength()) * KinematicMatrix::rotZ(-a_ElbowRoll) * KinematicMatrix::rotX(-a_ElbowYaw) * Hand2Elbow;

  // calculate WristYaw
  auto hand2handBaseRotM = Hand2HandBase.rotM.toRotationMatrix();
  a_WristYaw = atan2(hand2handBaseRotM(2, 1), hand2handBaseRotM(2, 2));

  if (a_WristYaw > NaoProvider::maxRange(L_WRIST_YAW))
    a_WristYaw = NaoProvider::maxRange(L_WRIST_YAW);
  else if (a_WristYaw < NaoProvider::minRange(L_WRIST_YAW))
    a_WristYaw = NaoProvider::minRange(L_WRIST_YAW);

  vector<float> arm;
  arm.push_back(a_ShoulderPitch);
  arm.push_back(a_ShoulderRoll);
  arm.push_back(a_ElbowYaw);
  arm.push_back(a_ElbowRoll);
  arm.push_back(a_WristYaw);
  arm.push_back(handOpening);

  return arm;
}


vector<float> InverseKinematics::getRArmAngles(const KinematicMatrix& desired, const float& handOpening)
{

  // Transformation of the desired hand position to shoulder space
  KinematicMatrix Hand2Shoulder =
      KinematicMatrix::transZ(-NaoProvider::link(SHOULDER_OFFSET_Z)) * KinematicMatrix::transY(NaoProvider::link(SHOULDER_OFFSET_Y)) * desired;

  // distance from shoulder to desired hand position
  float l = Hand2Shoulder.posV.norm();

  // normalized Vector from shoulder to desired hand position
  Vector3f n = Hand2Shoulder.posV / l;

  // declare ElbowRoll
  float a_ElbowRoll;

  // check, if the desired position is reachable
  if (l > NaoProvider::maxArmLength())
  {
    Hand2Shoulder.posV = n * NaoProvider::maxArmLength();
    l = NaoProvider::maxArmLength();
    a_ElbowRoll = NaoProvider::minRange(R_ELBOW_ROLL);
  }
  else if (l < NaoProvider::minArmLength())
  {
    Hand2Shoulder.posV = n * NaoProvider::minArmLength();
    l = NaoProvider::minArmLength();
    a_ElbowRoll = NaoProvider::maxRange(R_ELBOW_ROLL);
  }
  else
  {
    // rule of cosines
    a_ElbowRoll = -acos((pow(NaoProvider::link(UPPER_ARM_LENGTH), 2) + pow(NaoProvider::foreArmLength(), 2) - pow(l, 2)) /
                        (2 * NaoProvider::link(UPPER_ARM_LENGTH) * NaoProvider::foreArmLength())) +
                  (float)M_PI;
  }

  // calculation of the circles radius on which the elbow can be positioned
  float beta =
      acos((pow(l, 2) + pow(NaoProvider::link(UPPER_ARM_LENGTH), 2) - pow(NaoProvider::foreArmLength(), 2)) / (2 * l * NaoProvider::link(UPPER_ARM_LENGTH)));

  float r = sin(beta) * NaoProvider::link(UPPER_ARM_LENGTH);

  // distance from shoulder to circle midpoint
  float d = cos(beta) * NaoProvider::link(UPPER_ARM_LENGTH);

  // Elbow position from desired hand position and orientation
  KinematicMatrix Shoulder2Elbow = KinematicMatrix::transX(NaoProvider::foreArmLength()) * Hand2Shoulder.invert();

  KinematicMatrix Elbow2Shoulder = Shoulder2Elbow.invert();

  // distance from desired elbow position to circle surface
  float s = n.dot(Elbow2Shoulder.posV) - d;

  // projection of desired elbow position on circle surface
  Vector3f p = Elbow2Shoulder.posV - n * s;

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
  float a1 = atan2(m.y(), m.x());
  float a2 = atan2(m.z(), sqrtf(pow(m.x(), 2) + pow(m.z(), 2)));

  // Transformation matrix to circle space
  KinematicMatrix ToCirc = KinematicMatrix::rotZ(a1) * KinematicMatrix::rotY(-a2);

  Vector3f pToCirc = ToCirc.invert() * pReachable;

  float a3 = atan2(-pToCirc.y(), pToCirc.z());

  // orthogonal circle vectors
  Vector3f u = ToCirc * KinematicMatrix::rotX(a3) * Vector3f(0, r, 0);
  Vector3f v = ToCirc * KinematicMatrix::rotX(a3) * Vector3f(0, 0, r);

  // set step size for iteration
  int circleParts = 60;
  float step = 2 * (float)M_PI / circleParts;

  // constant for shoulder roll limits
  float k = cos(NaoProvider::maxRange(R_SHOULDER_PITCH));

  // iteration variables
  float t = 0;
  float bestDis = std::numeric_limits<float>::infinity();
  float bestT = t;
  bool noAvailableCirclePoint = true;
  bool optimumFound = false;

  float a_ShoulderRoll = 0.0f;
  float a_ShoulderPitch = 0.0f;
  float a_ElbowYaw = 0.0f;
  float a_WristYaw = 0.0f;
  KinematicMatrix Hand2Elbow;
  KinematicMatrix Hand2HandBase;

  // iterate on circle
  for (int i = 1; i <= circleParts; i++)
  {

    // check if desired p is reachable
    if (pReachable.y() <= NaoProvider::maxRElbowY() && pReachable.y() >= NaoProvider::minRElbowY() && pReachable.x() >= getPitchlimit(pReachable.y(), k))
    {
      noAvailableCirclePoint = false;

      a_ShoulderRoll = asin(pReachable.y() / NaoProvider::link(UPPER_ARM_LENGTH));

      a_ShoulderPitch = atan2(-pReachable.z(), pReachable.x());

      Hand2Elbow = KinematicMatrix::transX(-NaoProvider::link(UPPER_ARM_LENGTH)) * KinematicMatrix::rotZ(-a_ShoulderRoll) *
                   KinematicMatrix::rotY(-a_ShoulderPitch) * Hand2Shoulder;

      a_ElbowYaw = atan2(Hand2Elbow.posV.z(), Hand2Elbow.posV.y());

      // if ElbowYaw is in range, then optimum is found
      if (a_ElbowYaw <= NaoProvider::maxRange(R_ELBOW_YAW) && a_ElbowYaw >= NaoProvider::minRange(R_ELBOW_YAW))
      {
        optimumFound = true;
        break;
      }
      else
      {
        // store distance to desired hand position
        // set ElbowYaw in Range
        if (a_ElbowYaw > NaoProvider::maxRange(R_ELBOW_YAW))
          a_ElbowYaw = NaoProvider::maxRange(R_ELBOW_YAW);
        else
          a_ElbowYaw = NaoProvider::minRange(R_ELBOW_YAW);

        // transform to handbase space
        Hand2HandBase =
            KinematicMatrix::transX(-NaoProvider::foreArmLength()) * KinematicMatrix::rotZ(-a_ElbowRoll) * KinematicMatrix::rotX(-a_ElbowYaw) * Hand2Elbow;

        float dis = Hand2HandBase.posV.norm();

        // check if distance to desired hand is better than the best found solution yet
        if (dis < bestDis)
        {
          bestT = t;
          bestDis = dis;
        }
      }
    }

    // step
    t = t + i * step;
    // alternate
    step = -step;
    pReachable = m + u * sin(t) + v * cos(t);
  }


  // if no optimum could be found
  if (!optimumFound)
  {
    // if there was a possible elbow position on the circle
    if (!noAvailableCirclePoint)
      // take best t
      pReachable = m + u * sin(bestT) + v * cos(bestT);
    else
      // take the desired elbow position ( not on circle)
      pReachable = pDesired;

    a_ShoulderRoll = asin(pReachable.y() / NaoProvider::link(UPPER_ARM_LENGTH));

    a_ShoulderPitch = atan2(-pReachable.z(), pReachable.x());

    // check if the angles are in range
    if (a_ShoulderRoll > NaoProvider::maxRange(R_SHOULDER_ROLL))
      a_ShoulderRoll = NaoProvider::maxRange(R_SHOULDER_ROLL);
    else if (a_ShoulderRoll < NaoProvider::minRange(R_SHOULDER_ROLL))
      a_ShoulderRoll = NaoProvider::minRange(R_SHOULDER_ROLL);

    if (a_ShoulderPitch > NaoProvider::maxRange(R_SHOULDER_PITCH))
      a_ShoulderPitch = NaoProvider::maxRange(R_SHOULDER_PITCH);
    else if (a_ShoulderPitch < NaoProvider::minRange(R_SHOULDER_PITCH))
      a_ShoulderPitch = NaoProvider::minRange(R_SHOULDER_PITCH);

    Hand2Elbow = KinematicMatrix::transX(-NaoProvider::link(UPPER_ARM_LENGTH)) * KinematicMatrix::rotZ(-a_ShoulderRoll) *
                 KinematicMatrix::rotY(-a_ShoulderPitch) * Hand2Shoulder;

    a_ElbowYaw = atan2(Hand2Elbow.posV.z(), Hand2Elbow.posV.y());

    // check elbowYaw
    if (a_ElbowYaw > NaoProvider::maxRange(R_ELBOW_YAW))
      a_ElbowYaw = NaoProvider::maxRange(R_ELBOW_YAW);
    else if (a_ElbowYaw < NaoProvider::minRange(R_ELBOW_YAW))
      a_ElbowYaw = NaoProvider::minRange(R_ELBOW_YAW);
  }

  // transform to handbase space
  Hand2HandBase =
      KinematicMatrix::transX(-NaoProvider::foreArmLength()) * KinematicMatrix::rotZ(-a_ElbowRoll) * KinematicMatrix::rotX(-a_ElbowYaw) * Hand2Elbow;

  // calculate WristYaw
  auto hand2handBaseRotM = Hand2HandBase.rotM.toRotationMatrix();
  a_WristYaw = atan2(hand2handBaseRotM(2, 1), hand2handBaseRotM(2, 2));

  if (a_WristYaw > NaoProvider::maxRange(R_WRIST_YAW))
    a_WristYaw = NaoProvider::maxRange(R_WRIST_YAW);
  else if (a_WristYaw < NaoProvider::minRange(R_WRIST_YAW))
    a_WristYaw = NaoProvider::minRange(R_WRIST_YAW);

  vector<float> arm;
  arm.push_back(a_ShoulderPitch);
  arm.push_back(a_ShoulderRoll);
  arm.push_back(a_ElbowYaw);
  arm.push_back(a_ElbowRoll);
  arm.push_back(a_WristYaw);
  arm.push_back(handOpening);

  return arm;
}


vector<float> InverseKinematics::getFixedLLegAngles(const KinematicMatrix& desired, const float& a_HipYawPitch)
{

  // store hipyawpitch
  float hyp = a_HipYawPitch;

  // check if given HipYawPitch is in range
  if (hyp > NaoProvider::maxRange(L_HIP_YAW_PITCH))
    hyp = NaoProvider::maxRange(L_HIP_YAW_PITCH);
  else if (hyp < NaoProvider::minRange(L_HIP_YAW_PITCH))
    hyp = NaoProvider::minRange(L_HIP_YAW_PITCH);


  // First we need the torso position from foot space to calculate the desired angle position
  KinematicMatrix torso2ankle = KinematicMatrix::transZ(-NaoProvider::link(FOOT_HEIGHT)) * desired.invert();

  // Invert to get the desired annkle position
  KinematicMatrix ankleDesired = torso2ankle.invert();

  // transformation of the desired ankle position to rotated hip space
  KinematicMatrix ankle2hipOrthogonal = KinematicMatrix::rotX(-45.0f * TO_RAD) * KinematicMatrix::transY(-NaoProvider::link(HIP_OFFSET_Y)) *
                                        KinematicMatrix::transZ(NaoProvider::link(HIP_OFFSET_Z)) * ankleDesired;

  // transformation to space rotated about fixed HipYawPitch angle
  KinematicMatrix ankle2RotatedHipOrthogonal = KinematicMatrix::rotZ(hyp) * ankle2hipOrthogonal;

  // distance from ankle to hip
  float l = ankle2RotatedHipOrthogonal.posV.norm();

  // normal vector to ankle
  Vector3f n = ankle2RotatedHipOrthogonal.posV / l;

  float a_KneePitch;

  // reachability check
  if (l > NaoProvider::maxLegLength())
  {
    ankle2RotatedHipOrthogonal.posV = n * NaoProvider::maxLegLength();
    l = NaoProvider::maxLegLength();
    a_KneePitch = 0.0f;
  }
  else if (l < NaoProvider::minLegLength())
  {
    ankle2RotatedHipOrthogonal.posV = n * NaoProvider::minLegLength();
    l = NaoProvider::minLegLength();
    a_KneePitch = NaoProvider::maxRange(L_KNEE_PITCH);
  }
  else
  {
    // calculation of kneePitch with rule of cosines
    a_KneePitch = (float)M_PI - acos((pow(NaoProvider::link(THIGH_LENGTH), 2) + pow(NaoProvider::link(TIBIA_LENGTH), 2) - pow(l, 2)) /
                                     (2 * NaoProvider::link(THIGH_LENGTH) * NaoProvider::link(TIBIA_LENGTH)));
  }

  // calculation of HipPitch from triangle and position of ankle
  float a_HipPitch =
      -(acos((pow(NaoProvider::link(THIGH_LENGTH), 2) - pow(NaoProvider::link(TIBIA_LENGTH), 2) + pow(l, 2)) / (2 * NaoProvider::link(THIGH_LENGTH) * l)) +
        asin(ankle2RotatedHipOrthogonal.posV.x() / l));

  // calculation of hip roll angle from position of ankle
  float a_HipRoll = atan2(ankle2RotatedHipOrthogonal.posV.z(), ankle2RotatedHipOrthogonal.posV.y()) + 3.0f / 4.0f * (float)M_PI;

  // hold hip angles in range
  if (a_HipPitch > NaoProvider::maxRange(L_HIP_PITCH))
    a_HipPitch = NaoProvider::maxRange(L_HIP_PITCH);
  else if (a_HipPitch < NaoProvider::minRange(L_HIP_PITCH))
    a_HipPitch = NaoProvider::minRange(L_HIP_PITCH);

  if (a_HipRoll > NaoProvider::maxRange(L_HIP_ROLL))
    a_HipRoll = NaoProvider::maxRange(L_HIP_ROLL);
  else if (a_HipRoll < NaoProvider::minRange(L_HIP_ROLL))
    a_HipRoll = NaoProvider::minRange(L_HIP_ROLL);

  // transformation to ankle space
  KinematicMatrix ankleRotated2ankle = KinematicMatrix::transZ(-NaoProvider::link(TIBIA_LENGTH)) * KinematicMatrix::rotY(a_KneePitch) *
                                       KinematicMatrix::transZ(-NaoProvider::link(THIGH_LENGTH)) * KinematicMatrix::rotY(a_HipPitch) *
                                       KinematicMatrix::rotX(-(a_HipRoll + 3.0f / 4.0f * (float)M_PI)) * ankle2RotatedHipOrthogonal;

  auto ankleRotationMatrix = ankleRotated2ankle.rotM.toRotationMatrix();
  float a_AnkleRoll = asin(ankleRotationMatrix(1, 2));
  float a_AnklePitch = -(atan2(-ankleRotationMatrix(0, 2), -ankleRotationMatrix(2,2)));

  // hold ankle angles in range
  if (a_AnklePitch > NaoProvider::maxRange(L_ANKLE_PITCH))
    a_AnklePitch = NaoProvider::maxRange(L_ANKLE_PITCH);
  else if (a_AnklePitch < NaoProvider::minRange(L_ANKLE_PITCH))
    a_AnklePitch = NaoProvider::minRange(L_ANKLE_PITCH);

  if (a_AnkleRoll > NaoProvider::maxRangeLAnkleRoll(a_AnklePitch))
    a_AnkleRoll = NaoProvider::maxRangeLAnkleRoll(a_AnklePitch);
  else if (a_AnkleRoll < NaoProvider::minRangeLAnkleRoll(a_AnklePitch))
    a_AnkleRoll = NaoProvider::minRangeLAnkleRoll(a_AnklePitch);


  vector<float> leg;
  leg.push_back(hyp);
  leg.push_back(a_HipRoll);
  leg.push_back(a_HipPitch);
  leg.push_back(a_KneePitch);
  leg.push_back(a_AnklePitch);
  leg.push_back(a_AnkleRoll);

  return leg;
}
vector<float> InverseKinematics::getFixedRLegAngles(const KinematicMatrix& desired, const float& a_HipYawPitch)
{

  // store hipyawpitch
  float hyp = a_HipYawPitch;

  // check if given HipYawPitch is in range
  if (hyp > NaoProvider::maxRange(R_HIP_YAW_PITCH))
    hyp = NaoProvider::maxRange(R_HIP_YAW_PITCH);
  else if (hyp < NaoProvider::minRange(R_HIP_YAW_PITCH))
    hyp = NaoProvider::minRange(R_HIP_YAW_PITCH);


  // First we need the torso position from foot space to calculate the desired angle position
  KinematicMatrix torso2ankle = KinematicMatrix::transZ(-NaoProvider::link(FOOT_HEIGHT)) * desired.invert();

  // Invert to get the desired annkle position
  KinematicMatrix ankleDesired = torso2ankle.invert();

  // transformation of the desired ankle position to rotated hip space
  KinematicMatrix ankle2hipOrthogonal = KinematicMatrix::rotX(45.0f * TO_RAD) * KinematicMatrix::transY(NaoProvider::link(HIP_OFFSET_Y)) *
                                        KinematicMatrix::transZ(NaoProvider::link(HIP_OFFSET_Z)) * ankleDesired;

  // transformation to space rotated about fixed HipYawPitch angle
  KinematicMatrix ankle2RotatedHipOrthogonal = KinematicMatrix::rotZ(-hyp) * ankle2hipOrthogonal;

  // distance from ankle to hip
  float l = ankle2RotatedHipOrthogonal.posV.norm();

  // normal vector to ankle
  Vector3f n = ankle2RotatedHipOrthogonal.posV / l;

  float a_KneePitch;

  // reachability check
  if (l > NaoProvider::maxLegLength())
  {
    ankle2RotatedHipOrthogonal.posV = n * NaoProvider::maxLegLength();
    l = NaoProvider::maxLegLength();
    a_KneePitch = 0.0f;
  }
  else if (l < NaoProvider::minLegLength())
  {
    ankle2RotatedHipOrthogonal.posV = n * NaoProvider::minLegLength();
    l = NaoProvider::minLegLength();
    a_KneePitch = NaoProvider::maxRange(R_KNEE_PITCH);
  }
  else
  {
    // calculation of kneePitch with rule of cosines
    a_KneePitch = (float)M_PI - acos((pow(NaoProvider::link(THIGH_LENGTH), 2) + pow(NaoProvider::link(TIBIA_LENGTH), 2) - pow(l, 2)) /
                                     (2 * NaoProvider::link(THIGH_LENGTH) * NaoProvider::link(TIBIA_LENGTH)));
  }

  // calculation of HipPitch from triangle and position of ankle
  float a_HipPitch =
      -(acos((pow(NaoProvider::link(THIGH_LENGTH), 2) - pow(NaoProvider::link(TIBIA_LENGTH), 2) + pow(l, 2)) / (2 * NaoProvider::link(THIGH_LENGTH) * l)) +
        asin(ankle2RotatedHipOrthogonal.posV.x() / l));

  // calculation of hip roll angle from position of ankle
  float a_HipRoll = atan2(ankle2RotatedHipOrthogonal.posV.z(), ankle2RotatedHipOrthogonal.posV.y()) + 1.0f / 4.0f * (float)M_PI;

  // hold hip angles in range
  if (a_HipPitch > NaoProvider::maxRange(R_HIP_PITCH))
    a_HipPitch = NaoProvider::maxRange(R_HIP_PITCH);
  else if (a_HipPitch < NaoProvider::minRange(R_HIP_PITCH))
    a_HipPitch = NaoProvider::minRange(R_HIP_PITCH);

  if (a_HipRoll > NaoProvider::maxRange(R_HIP_ROLL))
    a_HipRoll = NaoProvider::maxRange(R_HIP_ROLL);
  else if (a_HipRoll < NaoProvider::minRange(R_HIP_ROLL))
    a_HipRoll = NaoProvider::minRange(R_HIP_ROLL);

  // transformation to ankle space
  KinematicMatrix ankleRotated2ankle = KinematicMatrix::transZ(NaoProvider::link(TIBIA_LENGTH)) * KinematicMatrix::rotY(-a_KneePitch) *
                                       KinematicMatrix::transZ(NaoProvider::link(THIGH_LENGTH)) * KinematicMatrix::rotY(-a_HipPitch) *
                                       KinematicMatrix::rotX(-(a_HipRoll + 1.0f / 4.0f * (float)M_PI)) * ankle2RotatedHipOrthogonal;

  auto ankleRotationMatrix = ankleRotated2ankle.rotM.toRotationMatrix();
  float a_AnkleRoll = -asin(ankleRotationMatrix(1, 2));
  float a_AnklePitch = -(atan2(-ankleRotationMatrix(0, 2), ankleRotationMatrix(2,2)));

  // hold ankle angles in range
  if (a_AnklePitch > NaoProvider::maxRange(R_ANKLE_PITCH))
    a_AnklePitch = NaoProvider::maxRange(R_ANKLE_PITCH);
  else if (a_AnklePitch < NaoProvider::minRange(R_ANKLE_PITCH))
    a_AnklePitch = NaoProvider::minRange(R_ANKLE_PITCH);

  if (a_AnkleRoll > NaoProvider::maxRangeRAnkleRoll(a_AnklePitch))
    a_AnkleRoll = NaoProvider::maxRangeRAnkleRoll(a_AnklePitch);
  else if (a_AnkleRoll < NaoProvider::minRangeRAnkleRoll(a_AnklePitch))
    a_AnkleRoll = NaoProvider::minRangeRAnkleRoll(a_AnklePitch);


  vector<float> leg;
  leg.push_back(hyp);
  leg.push_back(a_HipRoll);
  leg.push_back(a_HipPitch);
  leg.push_back(a_KneePitch);
  leg.push_back(a_AnklePitch);
  leg.push_back(a_AnkleRoll);

  return leg;
}
float InverseKinematics::getPitchlimit(const float& y, const float& k)
{
  return k * sqrt(pow(NaoProvider::link(UPPER_ARM_LENGTH), 2) - pow(y, 2));
}
