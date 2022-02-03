#pragma once

#include "Hardware/Definitions.hpp"

namespace JointUtils
{

  template <typename T>
  constexpr void fillHead(JointsArray<T>& destination, const JointsHeadArray<T>& head)
  {
    destination[Joints::HEAD_YAW] = head[JointsHead::YAW];
    destination[Joints::HEAD_PITCH] = head[JointsHead::PITCH];
  }

  template <typename T>
  constexpr void fillLeftArm(JointsArray<T>& destination, const JointsArmArray<T>& src)
  {
    destination[Joints::L_SHOULDER_PITCH] = src[JointsArm::SHOULDER_PITCH];
    destination[Joints::L_SHOULDER_ROLL] = src[JointsArm::SHOULDER_ROLL];
    destination[Joints::L_ELBOW_YAW] = src[JointsArm::ELBOW_YAW];
    destination[Joints::L_ELBOW_ROLL] = src[JointsArm::ELBOW_ROLL];
    destination[Joints::L_WRIST_YAW] = src[JointsArm::WRIST_YAW];
    destination[Joints::L_HAND] = src[JointsArm::HAND];
  }

  template <typename T>
  constexpr void fillRightArm(JointsArray<T>& destination, const JointsArmArray<T>& src)
  {
    destination[Joints::R_SHOULDER_PITCH] = src[JointsArm::SHOULDER_PITCH];
    destination[Joints::R_SHOULDER_ROLL] = src[JointsArm::SHOULDER_ROLL];
    destination[Joints::R_ELBOW_YAW] = src[JointsArm::ELBOW_YAW];
    destination[Joints::R_ELBOW_ROLL] = src[JointsArm::ELBOW_ROLL];
    destination[Joints::R_WRIST_YAW] = src[JointsArm::WRIST_YAW];
    destination[Joints::R_HAND] = src[JointsArm::HAND];
  }

  template <typename T>
  constexpr void fillArms(JointsArray<T>& destination, const JointsArmArray<T>& leftArm,
                          const JointsArmArray<T>& rightArm)
  {
    fillLeftArm(destination, leftArm);
    fillRightArm(destination, rightArm);
  }

  template <typename T>
  constexpr void fillLeftLeg(JointsArray<T>& destination, const JointsLegArray<T>& src)
  {
    destination[Joints::L_HIP_YAW_PITCH] = src[JointsLeg::HIP_YAW_PITCH];
    destination[Joints::L_HIP_ROLL] = src[JointsLeg::HIP_ROLL];
    destination[Joints::L_HIP_PITCH] = src[JointsLeg::HIP_PITCH];
    destination[Joints::L_KNEE_PITCH] = src[JointsLeg::KNEE_PITCH];
    destination[Joints::L_ANKLE_PITCH] = src[JointsLeg::ANKLE_PITCH];
    destination[Joints::L_ANKLE_ROLL] = src[JointsLeg::ANKLE_ROLL];
  }

  template <typename T>
  constexpr void fillRightLeg(JointsArray<T>& destination, const JointsLegArray<T>& src)
  {
    destination[Joints::R_HIP_YAW_PITCH] = src[JointsLeg::HIP_YAW_PITCH];
    destination[Joints::R_HIP_ROLL] = src[JointsLeg::HIP_ROLL];
    destination[Joints::R_HIP_PITCH] = src[JointsLeg::HIP_PITCH];
    destination[Joints::R_KNEE_PITCH] = src[JointsLeg::KNEE_PITCH];
    destination[Joints::R_ANKLE_PITCH] = src[JointsLeg::ANKLE_PITCH];
    destination[Joints::R_ANKLE_ROLL] = src[JointsLeg::ANKLE_ROLL];
  }

  template <typename T>
  constexpr void fillLegs(JointsArray<T>& destination, const JointsLegArray<T>& leftLeg,
                          const JointsLegArray<T>& rightLeg)
  {
    fillLeftLeg(destination, leftLeg);
    fillRightLeg(destination, rightLeg);
  }

  template <typename T>
  constexpr JointsHeadArray<T> extractHead(const JointsArray<T>& src)
  {
    return {{src[Joints::HEAD_YAW], src[Joints::HEAD_PITCH]}};
  }

  template <typename T>
  constexpr JointsArmArray<T> extractLeftArm(const JointsArray<T>& src)
  {
    return {{src[Joints::L_SHOULDER_PITCH], src[Joints::L_SHOULDER_ROLL], src[Joints::L_ELBOW_YAW],
             src[Joints::L_ELBOW_ROLL], src[Joints::L_WRIST_YAW], src[Joints::L_HAND]}};
  }

  template <typename T>
  constexpr JointsArmArray<T> extractRightArm(const JointsArray<T>& src)
  {
    return {{src[Joints::R_SHOULDER_PITCH], src[Joints::R_SHOULDER_ROLL], src[Joints::R_ELBOW_YAW],
             src[Joints::R_ELBOW_ROLL], src[Joints::R_WRIST_YAW], src[Joints::R_HAND]}};
  }

  template <typename T>
  constexpr JointsLegArray<T> extractLeftLeg(const JointsArray<T>& src)
  {
    return {{src[Joints::L_HIP_YAW_PITCH], src[Joints::L_HIP_ROLL], src[Joints::L_HIP_PITCH],
             src[Joints::L_KNEE_PITCH], src[Joints::L_ANKLE_PITCH], src[Joints::L_ANKLE_ROLL]}};
  }

  template <typename T>
  constexpr JointsLegArray<T> extractRightLeg(const JointsArray<T>& src)
  {
    return {{src[Joints::R_HIP_YAW_PITCH], src[Joints::R_HIP_ROLL], src[Joints::R_HIP_PITCH],
             src[Joints::R_KNEE_PITCH], src[Joints::R_ANKLE_PITCH], src[Joints::R_ANKLE_ROLL]}};
  }
} // namespace JointUtils
