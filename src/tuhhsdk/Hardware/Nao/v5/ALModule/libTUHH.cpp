#include <chrono>

#include <alcommon/albrokermanager.h>
#include <boost/interprocess/sync/scoped_lock.hpp>

#include "Hardware/Nao/common/SMO.h"

#include "DcmConnectorAL.h"

#include "libTUHH.h"


libTUHH* libTUHH::instance = NULL;

using namespace boost::interprocess;


#ifdef _WIN32
# define ALCALL __declspec(dllexport)
#else
# define ALCALL
#endif

extern "C"
{
ALCALL int _createModule(boost::shared_ptr<AL::ALBroker> pBroker)
{
  // init broker with the main broker instance
  // from the parent executable
  AL::ALBrokerManager::setInstance(pBroker->fBrokerManager.lock());
  AL::ALBrokerManager::getInstance()->addBroker(pBroker);

  // create module instances
  AL::ALModule::createModule<libTUHH>(pBroker, "libTUHH");

  return 0;
}

ALCALL int _closeModule()
{
  return 0;
}

} // extern "C"



// libTUHH
libTUHH::libTUHH(boost::shared_ptr<AL::ALBroker> broker, const std::string &name) :
  AL::ALModule(broker, name),
  shmBlock_(NULL),
  broker(broker),
  previousChestButton(0.0f),
  sentChestButton(true),
  droppedFrame(false)
{
  assert(instance == NULL);
  instance = this;

  try {
    segment_ = shared_memory_object(create_only, SMO::shmName, read_write);
    segment_.truncate(SMO::shmSize);

    std::cout << "\033[0;34m[SHM_INFO\t]\033[0m " << "Shared memory created!\n";

    // Construct shared block
    region_ = mapped_region(segment_, read_write);
    std::cout << "\033[0;34m[SHM_INFO\t]\033[0m " << "Shared memory mapped!\n";
    // This is placement-new. It constructs the shared block at a specified address.
    shmBlock_ = new (region_.get_address()) SharedBlock;
  }
  catch (const interprocess_exception& e)
  {
    std::cout << "\033[0;31m[SHM_ERROR\t]\033[0m " << e.what() << std::endl;
  }
}

libTUHH::~libTUHH()
{
  instance = NULL;
}

void libTUHH::init()
{
  dcmProxy = broker->getDcmProxy();
  DcmConnectorAL::init(broker);

  try {

    // Joints
    for (int i = 0; i < keys::joints::JOINTS_MAX; i++)
    {
      jointSensor[i]	    = DcmConnectorAL::getDataPtr(keys::joints::sensorKey[i]);
      jointCurrent[i]	    = DcmConnectorAL::getDataPtr(keys::joints::currentKey[i]);
      jointTemperature[i] = DcmConnectorAL::getDataPtr(keys::joints::temperatureKey[i]);
      jointStatus[i]      = DcmConnectorAL::getDataPtr(keys::joints::statusKey[i]);
    }
    // Sensors
    for (int i = 0; i <keys::sensor::SWITCH_MAX; i++)
    {
      switches[i] = DcmConnectorAL::getDataPtr(keys::sensor::switchKey[i]);
    }
    // IMU
    for (int i= 0; i< keys::sensor::IMU_MAX; i++)
    {
      imu[i] = DcmConnectorAL::getDataPtr(keys::sensor::imuKey[i]);
    }
    // FSR
    for (int i = 0; i < keys::sensor::FSR_MAX; i++)
    {
      fsrLeft[i]	= DcmConnectorAL::getDataPtr(keys::sensor::fsrLeftKey[i]);
      fsrRight[i]	= DcmConnectorAL::getDataPtr(keys::sensor::fsrRightKey[i]);
    }
    // Sonar
    for (int i= 0; i < keys::sensor::SONAR_MAX; i++)
    {
      sonar[i] = DcmConnectorAL::getDataPtr(keys::sensor::sonarKey[i]);
    }
    // Battery
    for (int i = 0; i < keys::sensor::BATTERY_MAX; i++)
    {
      battery[i] = DcmConnectorAL::getDataPtr(keys::sensor::batteryKey[i]);
    }
    // NAO
    scoped_lock<SharedBlock::mutex_t> lock(shmBlock_->accessMutex);
    for (int i = 0; i < keys::naoinfos::NAOINFO_MAX; i++)
    {
      std::string info = DcmConnectorAL::getDataString(keys::naoinfos::naoInfoKey[i]);
      // Only the last four bytes of the body ID are relevant. Some NAOs have a \r character in the end.
      if (i == keys::naoinfos::BODY_ID && info.length() > 5)
      {
        if (info.at(info.length() - 1) == 13)
        {
          info = info.substr(info.length() - 5, 4);
        }
        else
        {
          info = info.substr(info.length() - 4);
        }
      }
      if (info.size() < 64)
      {
        strcpy(shmBlock_->naoInfoKey[i], info.c_str());
      }
    }
  }
  catch ( ... )
  {
    std::cout << "\033[0;31m[SHM_ERROR\t]\033[0m " << "Unknown exception in libTUHH::init()\n";
  }

  // Create the JointActuatorBody and JointHardnessBody aliases
  std::vector<std::string> jab(keys::joints::JOINTS_MAX+1);
  std::vector<std::string> jhb(keys::joints::JOINTS_MAX+1);
  jab[0] = "JointActuatorBody";
  jhb[0] = "JointHardnessBody";
  for (int i = 0; i < keys::joints::JOINTS_MAX; i++)
  {
    jab[i+1] = keys::joints::actuatorKey[i];
    jhb[i+1] = keys::joints::hardnessKey[i];
  }
  DcmConnectorAL::createAlias(jab);
  DcmConnectorAL::createAlias(jhb);
  // Create the LEDKeys alias
  std::vector<std::string> ledkeys;
  ledkeys.push_back("LEDKeys");
  for(int i = 0; i < keys::led::CHEST_MAX; ++i) {
    ledkeys.push_back(keys::led::chestKey[i]);
  }
  for(int i = 0; i < keys::led::EAR_MAX; ++i) {
    ledkeys.push_back(keys::led::earLeftKey[i]);
  }
  for(int i = 0; i < keys::led::EAR_MAX; ++i) {
    ledkeys.push_back(keys::led::earRightKey[i]);
  }
  for(int i = 0; i < keys::led::EYE_MAX; ++i) {
    ledkeys.push_back(keys::led::eyeLeftKey[i]);
  }
  for(int i = 0; i < keys::led::EYE_MAX; ++i) {
    ledkeys.push_back(keys::led::eyeRightKey[i]);
  }

  for(int i = 0; i < keys::led::FOOT_MAX; ++i) {
    ledkeys.push_back(keys::led::footLeftKey[i]);
  }
  for(int i = 0; i < keys::led::FOOT_MAX; ++i) {
    ledkeys.push_back(keys::led::footRightKey[i]);
  }
  DcmConnectorAL::createAlias(ledkeys);

  // head keys used for displaying battery status; ordered as circle
  std::vector<std::string> batterykeys;
  batterykeys.push_back("BatteryKeys");
  batterykeys.push_back(keys::led::headKey[keys::led::HEAD_FRONT_LEFT_1]);
  batterykeys.push_back(keys::led::headKey[keys::led::HEAD_FRONT_LEFT_0]);
  batterykeys.push_back(keys::led::headKey[keys::led::HEAD_MIDDLE_LEFT_0]);
  batterykeys.push_back(keys::led::headKey[keys::led::HEAD_REAR_LEFT_0]);
  batterykeys.push_back(keys::led::headKey[keys::led::HEAD_REAR_LEFT_1]);
  batterykeys.push_back(keys::led::headKey[keys::led::HEAD_REAR_LEFT_2]);
  batterykeys.push_back(keys::led::headKey[keys::led::HEAD_REAR_RIGHT_2]);
  batterykeys.push_back(keys::led::headKey[keys::led::HEAD_REAR_RIGHT_1]);
  batterykeys.push_back(keys::led::headKey[keys::led::HEAD_REAR_RIGHT_0]);
  batterykeys.push_back(keys::led::headKey[keys::led::HEAD_MIDDLE_RIGHT_0]);
  batterykeys.push_back(keys::led::headKey[keys::led::HEAD_FRONT_RIGHT_0]);
  batterykeys.push_back(keys::led::headKey[keys::led::HEAD_FRONT_RIGHT_1]);
  DcmConnectorAL::createAlias(batterykeys);

  angleCommand.arraySetSize(6);
  angleCommand[0] = std::string("JointActuatorBody");
  angleCommand[1] = std::string("ClearAll");
  angleCommand[2] = std::string("time-separate");
  angleCommand[3] = 0;
  angleCommand[4].arraySetSize(1);
  angleCommand[5].arraySetSize(shmBlock_->commandAngles.size());
  for (unsigned int i = 0; i < shmBlock_->commandAngles.size(); i++) {
    angleCommand[5][i].arraySetSize(1);
  }

  stiffnessCommand.arraySetSize(6);
  stiffnessCommand[0] = std::string("JointHardnessBody");
  stiffnessCommand[1] = std::string("ClearAll");
  stiffnessCommand[2] = std::string("time-separate");
  stiffnessCommand[3] = 0;
  stiffnessCommand[4].arraySetSize(1);
  stiffnessCommand[5].arraySetSize(shmBlock_->commandStiffnesses.size());
  for (unsigned int i = 0; i < shmBlock_->commandStiffnesses.size(); i++) {
    stiffnessCommand[5][i].arraySetSize(1);
  }

  ledCommand.arraySetSize(6);
  ledCommand[0] = std::string("LEDKeys");
  ledCommand[1] = std::string("ClearAll");
  ledCommand[2] = std::string("time-separate");
  ledCommand[3] = 0;
  ledCommand[4].arraySetSize(1);
  ledCommand[5].arraySetSize(shmBlock_->commandLEDs.size());
  for (unsigned int i = 0; i < shmBlock_->commandLEDs.size(); i++) {
    ledCommand[5][i].arraySetSize(1);
  }

  batteryCommand.arraySetSize(6);
  batteryCommand[0] = std::string("BatteryKeys");
  batteryCommand[1] = std::string("ClearAll");
  batteryCommand[2] = std::string("time-separate");
  batteryCommand[3] = 0;
  batteryCommand[4].arraySetSize(1);
  batteryCommand[5].arraySetSize(keys::led::HEAD_MAX);
  for (unsigned int i = 0; i < keys::led::HEAD_MAX; i++) {
    batteryCommand[5][i].arraySetSize(1);
  }

  sonarCommand.arraySetSize(3);
  sonarCommand[0] = std::string(keys::sensor::sonarKey[keys::sensor::SONAR_ACTUATOR]);
  sonarCommand[1] = std::string("Merge");
  sonarCommand[2].arraySetSize(1);
  sonarCommand[2][0].arraySetSize(2);

  // connect cycle-function to DCM-Process
  preCycleConnection = DcmConnectorAL::bindPre(boost::bind(&libTUHH::staticPreCycle));
  postCycleConnection = DcmConnectorAL::bindPost(boost::bind(&libTUHH::staticPostCycle));
}

void libTUHH::exit()
{
  preCycleConnection.disconnect();
  postCycleConnection.disconnect();
}

void libTUHH::preCycle()
{
  {
    // Evaluate commands received through shared memory
    scoped_lock<SharedBlock::mutex_t> lock(shmBlock_->accessMutex);

    int dcmTime = DcmConnectorAL::getTime();
    if (shmBlock_->newAngles) {
      angleCommand[4][0] = dcmTime;
      for (unsigned int i = 0; i < shmBlock_->commandAngles.size(); i++) {
        angleCommand[5][i][0] = shmBlock_->commandAngles[i];
      }
      dcmProxy->setAlias(angleCommand);
      shmBlock_->newAngles = false;
    }
    if (shmBlock_->newStiffnesses) {
      stiffnessCommand[4][0] = dcmTime;
      for (unsigned int i = 0; i < shmBlock_->commandStiffnesses.size(); i++) {
        stiffnessCommand[5][i][0] = shmBlock_->commandStiffnesses[i];
      }
      dcmProxy->setAlias(stiffnessCommand);
      shmBlock_->newStiffnesses = false;
    }
    batteryDisplay.displayBatteryCharge(*battery[keys::sensor::BATTERY_CHARGE], *battery[keys::sensor::BATTERY_CURRENT], batteryLEDs);
    if (shmBlock_->newLEDs) {
      ledCommand[4][0] = dcmTime;
      for (unsigned int i = 0; i < shmBlock_->commandLEDs.size(); i++) {
        if (i < keys::led::CHEST_MAX + 2 * keys::led::EAR_MAX + 2 * keys::led::EYE_MAX) {
          ledCommand[5][i][0] = shmBlock_->commandLEDs[i];
        } else if (i < keys::led::CHEST_MAX + 2 * keys::led::EAR_MAX + 2 * keys::led::EYE_MAX + keys::led::HEAD_MAX) {
          // The head has to be skipped becaused it is not accessible in this way anymore.
        } else {
          ledCommand[5][i - keys::led::HEAD_MAX][0] = shmBlock_->commandLEDs[i];
        }
      }
      dcmProxy->setAlias(ledCommand);
      shmBlock_->newLEDs = false;
    } else {
      for (unsigned int i = 0; i < keys::led::HEAD_MAX; i++) {
        if (static_cast<float>(batteryCommand[5][i][0]) != batteryLEDs[i]) {
          batteryCommand[4][0] = dcmTime;
          for (unsigned int j = i; j < keys::led::HEAD_MAX; j++) {
            batteryCommand[5][j][0] = batteryLEDs[j];
          }
          dcmProxy->setAlias(batteryCommand);
          break;
        }
      }
    }
    if (shmBlock_->newSonar) {
      sonarCommand[2][0][0] = shmBlock_->commandSonar;
      sonarCommand[2][0][1] = dcmTime;
      dcmProxy->set(sonarCommand);
      shmBlock_->newSonar = false;
    }
  }
}

void libTUHH::postCycle()
{
  {
    scoped_lock<SharedBlock::mutex_t> lock(shmBlock_->accessMutex);
    // Joints
    for (int i = 0; i < keys::joints::JOINTS_MAX; ++i)
    {
      shmBlock_->jointSensor[i]      = *jointSensor[i];
      shmBlock_->jointCurrent[i]	   = *jointCurrent[i];
      shmBlock_->jointTemperature[i] = *jointTemperature[i];
      shmBlock_->jointStatus[i]      = *jointStatus[i];
    }
    // Sensors
    for (int i = 0; i <keys::sensor::SWITCH_MAX; i++)
    {
      shmBlock_->switches[i] = *switches[i];
    }
    // IMU
    for (int i= 0; i< keys::sensor::IMU_MAX; i++)
    {
      shmBlock_->imu[i] = *imu[i];
    }
    // FSRERROR
    for (int i = 0; i < keys::sensor::FSR_MAX; i++)
    {
      shmBlock_->fsrLeft[i]	= *fsrLeft[i];
      shmBlock_->fsrRight[i]	= *fsrRight[i];
    }
    // Sonar
    for (int i= 0; i < keys::sensor::SONAR_MAX; i++)
    {
      shmBlock_->sonar[i] = *sonar[i];
    }
    // Battery
    for (int i = 0; i < keys::sensor::BATTERY_MAX; i++)
    {
      shmBlock_->battery[i] = *battery[i];
    }
    shmBlock_->time = std::chrono::duration_cast<std::chrono::duration<int64_t, std::nano>>(std::chrono::system_clock::now().time_since_epoch()).count();

    // CallbackBuffer
    {
      // Chest Button handling
      float currentButton = shmBlock_->switches[keys::sensor::SWITCH_CHEST_BUTTON];
      std::chrono::time_point<std::chrono::system_clock> now = std::chrono::system_clock::now();
      unsigned int msSinceLastChestButtonTime = std::chrono::duration_cast<std::chrono::milliseconds>(now - previousChestButtonTime).count();
      if (currentButton > previousChestButton) {
        if (msSinceLastChestButtonTime < 500) {
          shmBlock_->chestButtonDoublePressed = true;
          sentChestButton = true;
        } else {
          shmBlock_->chestButtonDoublePressed = false;
          sentChestButton = false;
        }
        shmBlock_->chestButtonPressed = false;
        previousChestButtonTime = now;
      } else if (msSinceLastChestButtonTime >= 500 && !sentChestButton) {
        shmBlock_->chestButtonPressed = true;
        shmBlock_->chestButtonDoublePressed = false;
        sentChestButton = true;
      } else {
        shmBlock_->chestButtonPressed = false;
        shmBlock_->chestButtonDoublePressed = false;
      }
      previousChestButton = currentButton;
    }
  }
  if (shmBlock_->semaphore.getCounter() < 1) {
    shmBlock_->semaphore.post();
    if (droppedFrame) {
      std::cout << "\033[0;34m[SHM_INFO\t]\033[0m Resynced after frame drop\n";
    }
    droppedFrame = false;
  } else {
    if (!droppedFrame) {
      std::cout << "\033[0;31m[SHM_ERROR\t]\033[0m Dropped frame\n";
      droppedFrame = true;
    }
  }
}

void libTUHH::staticPreCycle()
{
  assert(instance != NULL);
  instance->preCycle();
}

void libTUHH::staticPostCycle()
{
  assert(instance != NULL);
  instance->postCycle();
}
