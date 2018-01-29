#pragma once

#include <condition_variable>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <unordered_map>
#include <vector>

#include "DebugData.h"
#include "DebugTransportInterface.h"


class Image;

/**
 * This singleton class enables all programmers, to simply connect data to a key in this
 * Debug class. The data will be collected and hold up to date and will be sent via different
 * Transporters to different endpoints.
 * @author Robert Oehlmann
 * @author Finn Poppinga
 */
class Debug
{
private:
  Debug() = default;
  Debug(Debug const&) = delete;
  ~Debug() = default;
  Debug& operator=(const Debug&) = delete; //[Sad face operator] ~Poppinga

  void updateHelper(const std::string& key, const Uni::Value& value);

  void addTransport(const std::shared_ptr<DebugTransportInterface>& transport);
  void removeAllTransports();

  void start();
  void stop();
  void run();
  void trigger();

  std::unordered_map<std::string, unsigned int> keys_;

  std::mutex debugMutex_;
  std::mutex keysMutex_;
  std::vector<std::shared_ptr<DebugTransportInterface>> transporter_;
  std::thread transporterThread_;
  bool trigger_;
  bool shutdownThread_;
  std::condition_variable transporterCondition_;
  std::mutex transporterMutex_;

  friend class TUHH;
  friend class ThreadBase;
  friend struct std::default_delete<Debug>;

public:
  template <typename T>
  void update(const std::string& key, const T& value)
  {
    {
      std::lock_guard<std::mutex> l(keysMutex_);
      auto it = keys_.find(key);
      if (it == keys_.end())
      {
        keys_[key] = 0;
      }
      else if (it->second == 0)
      {
        return;
      }
    }
    Uni::Value v;
    v << value;
    updateHelper(key, v);
  }

  bool isSubscribed(const std::string& key)
  {
    std::lock_guard<std::mutex> l(keysMutex_);
    auto it = keys_.find(key);
    // The following line is written in a way that this method will return true until an update or sendImage has been tried.
    // After the first update/sendImage, it will be != keys_.end() and it->second will be == 0, so this will return false.
    return (it == keys_.end()) || (it->second > 0);
  }

  void subscribe(const std::string& key);
  void unsubscribe(const std::string& key);

  void pushQueue(const std::string& key, const std::string& message);

  void sendImage(const std::string& key, const Image& img);
};
