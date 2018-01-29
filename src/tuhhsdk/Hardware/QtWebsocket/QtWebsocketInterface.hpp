#pragma once

#include <mutex>
#include <vector>
#include <queue>
#include <string>
#include <array>
#include <thread>
#include <condition_variable>

#include <boost/bind.hpp>
#include <boost/shared_ptr.hpp>
#include <boost/enable_shared_from_this.hpp>
#include <boost/asio.hpp>
#include "Definitions/windows_definition_fix.hpp"

#include <QObject>
#include <QtWebSockets/QWebSocketServer>
#include <QtCore/QList>
#include <QtCore/QByteArray>
//#include "QtWebsocketInterface.moc"

QT_FORWARD_DECLARE_CLASS(QWebSocketServer)
QT_FORWARD_DECLARE_CLASS(QWebSocket)

#include "Tools/Math/Vector3.h"

#include "Hardware/RobotInterface.hpp"
#include "QtWebsocketCamera.hpp"
#include "print.h"

//////////////////
/// \brief The TcpServer class
///

class QtWebsocketInterface;

class TcpServer : public QObject {
    Q_OBJECT

public:
  TcpServer(QtWebsocketInterface* robotInterface, QObject * parent = 0);
  ~TcpServer();

public slots:
  void acceptConnection();
  void processTextMessage(QString message);
  void processBinaryMessage(QByteArray message);
  void socketDisconnected();
private:
  QWebSocketServer* server;
  QList<QWebSocket*> clients;
  QtWebsocketInterface* robotInterface;
};

//////////////////




class QtWebsocketInterface : public RobotInterface {
public:
  /**
   * @brief QWebsocketsInterface constructs a thread with a websocket server
   */
  QtWebsocketInterface(int argc, char** argv);
  /**
   * @brief configure does nothing
   */
  void configure(Configuration&);
  /**
   * @brief setJointAngles sets the joint angles for the current cycle
   * @param angles the values of all joint angles
   */
  void setJointAngles(const std::vector<float>& angles);
  /**
   * @brief setJointStiffnesses sets the joint stiffnesses for the current cycle
   * @param stiffnesses the values of all joint stiffnesses
   */
  void setJointStiffnesses(const std::vector<float>& stiffnesses);
  /**
   * @brief setLEDs sets the LED colors and/or brightnesses
   * @param leds the values of all LEDs
   */
  void setLEDs(const std::vector<float>& leds);
  /**
   * @brief setSonar sets the value of the sonar actuator
   * @param sonar the value of the sonar actuator (see Soft Bank documentation)
   */
  void setSonar(const float sonar);
  /**
   * @brief waitAndReadSensorData transmits joint commands, simulates a cycle and gets new sensor data
   * @param data is filled with sensor data from the websocket
   */
  void waitAndReadSensorData(NaoSensorData& data);
  /**
   * @brief getFileRoot returns a path to a directory that contains all files for our program
   * @return a path
   */
  std::string getFileRoot();
  /**
   * @brief getNaoInfo copies the hardware identification
   * @param info is filled with the body/head version and name
   */
  void getNaoInfo(Configuration&, NaoInfo& info);

  CameraInterface& getCamera(const Camera camera);
  CameraInterface& getCurrentCamera();

  void updateAccelData(float x, float y, float z);
  void updateGyroData(float x, float y, float z);

private:
    QtWebsocketCamera topCamera;
    QtWebsocketCamera bottomCamera;
    /// mutex to lock DCM commands of different threads
    std::mutex mutex_;
    /// accelerometer of simulated robot
    std::queue<Vector3<float>> accelerometer_;
    /// gyroscope of simulated robot
    std::queue<Vector3<float>> gyroscope_;

    std::thread qtMainThread;

    std::mutex updateLock;

    std::condition_variable sensorUpdateAvailable;
    std::mutex sensorUpdateMutex;
};
