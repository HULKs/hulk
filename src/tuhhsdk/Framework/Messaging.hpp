#pragma once

#include <typeindex>
#include <unordered_set>

#include "DataType.hpp"
#include "Tools/Var/SpscQueue.hpp"


struct Message
{
  /// the type of which the object in data is (needs to be initialized to something)
  std::type_index type = typeid(DataTypeBase);
  /// a pointer to an object containing a data type
  DataTypeBase* data;
};

typedef SpscRing<Message, 128> DataQueue;

class Sender
{
public:
  /**
   * @brief Sender creates a sender to a data queue
   * @param queue an already existing data queue
   */
  Sender(DataQueue& queue);
  /**
   * @brief send sends a message via this sender, it will be available at the receiver after this call
   * @param msg the message that is to be sent
   * @return true iff the message could be pushed to the queue
   */
  bool send(const Message& msg);
  /**
   * @brief getRequested returns a list of requested types
   * @return a list of the types could be sent via this sender (not all of them have to)
   */
  const std::vector<std::type_index>& getRequested() const;

private:
  /// list of requested types
  std::vector<std::type_index> requested_;
  /// the queue that this sender pushes messages to
  DataQueue& queue_;
  friend class Receiver;
};

class Receiver
{
public:
  /**
   * @brief Receiver creates a receiver from a sender
   * @param sender the sender that sends to this receiver
   */
  Receiver(Sender& sender);
  /**
   * @brief receive pops one message from the ingoing queue
   * @param msg is filled with the received message
   * @return true iff a message was available
   */
  bool receive(Message& msg);
  /**
   * @brief request announces that someone expects that a specific DataType comes out of this receiver
   * @param type the type that is requested
   */
  void request(const std::type_index& type);

private:
  /// the sender that sends to this receiver
  Sender& sender_;
};

class DuplexChannel
{
public:
  /**
   * @brief DuplexChannel creates data queues, senders and receivers for communication between two endpoints in both directions
   */
  DuplexChannel();
  /**
   * @brief getA2BSender get a sender
   * @return the sender that sends to B
   */
  Sender& getA2BSender();
  /**
   * @brief getA2BReceiver get a receiver
   * @return the receiver that receives from A
   */
  Receiver& getA2BReceiver();
  /**
   * @brief getB2ASender get a sender
   * @return the sender that sends to A
   */
  Sender& getB2ASender();
  /**
   * @brief getB2AReceiver get a receiver
   * @return the receiver that receives from B
   */
  Receiver& getB2AReceiver();

private:
  /// the queue from A to B
  DataQueue a2b_;
  /// the queue from B to A
  DataQueue b2a_;
  /// the sender that sends to B
  Sender a2b_sender_;
  /// the receiver that receives from A
  Receiver a2b_receiver_;
  /// the sender that sends to A
  Sender b2a_sender_;
  /// the receiver that receives from B
  Receiver b2a_receiver_;
};
