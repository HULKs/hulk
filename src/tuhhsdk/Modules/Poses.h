#pragma once

#include <string>
#include <vector>


/**
 * This class handles robot poses
 * Poses are stored in the Poses folder. For Example <a href="Home.pose">Home.pose</a>
 * @author <a href="mailto:stefan.kaufmann@tu-harburg.de">Stefan Kaufmann</a>
 */
class Poses
{
public:
  enum EnumPose
  {
    AL_INIT,
    ARMBACK,
    HOME,
    PENALIZED,
    READY,
    TAKEAWAY,
    TRANSPORT,
    POSE_MAX
  };
  /**
   * @brief getPose get a pose
   * @param index the index of a pose
   * @return a vector of joint angles
   */
  static const std::vector<float>& getPose(const EnumPose index);

private:
  /**
   * @brief init loads all pose files
   * @param fileRoot the directory which contains the poses directory
   * @return whether initialization was successful
   */
  static bool init(const std::string& fileRoot);
  /// the names of the files - must correspond to the order of EnumPose
  static const char* poseFiles[POSE_MAX];
  /// the joint angles for each pose
  static std::vector<float> poses[POSE_MAX];
  friend class TUHH;
};
