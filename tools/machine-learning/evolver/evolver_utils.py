import random
import math
import json
import os
import tensorflow as tf
import nn_utils
from numpy import array
import uuid


class Individual(object):
    def __init__(self,
                 name = "Bob-0",
                 score = 0.0,
                 fitness = 0.0,
                 cost = 0.0,
                 genes = {"trainingEpochs": 2,
                          "conv_layers": [],
                          "dense_layers": [],
                          "final_layer": ("sigmoid", True),
                          "optimizer": "adam",
                          "initial_learning_rate": 0.1,
                          "learning_rate_factor_per_epoch": 0.1}):
        self.name = name
        self.score = score
        self.fitness = fitness
        self.cost = cost
        self.genes = genes


class Population(object):
    def  __init__(self,
                  name = "Bobs",
                  best_fitness = Individual(),
                  best_score = Individual(),
                  members = [],
                  generation = 0,
                  min_size = 15,
                  max_size = 25,
                  nn_settings = {},
                  max_mutations_per_clone = 10,
                  progression = [],
                  trained_model_path = "",
                  population_path = "",
                  member_pad = None,
                  spam_pad = None,
                  progression_pad = None,
                  pop_pad = None,
                  status_pad = None,
                  conv_activation_functions = [],
                  dense_activation_functions = [],
                  final_activation_functions = [],
                  optimizers = [],
                  batch_size = 32,
                  tfr_train = None,
                  tfr_test = None,
                  tfr_verify = None,
                  predicter = None,
                  predicter_error_threshold = 0.01):
        self.name = name
        self.best_fitness = best_fitness
        self.best_score = best_score
        self.members = members
        self.generation = generation
        self.min_size = min_size
        self.max_size = max_size
        self.nn_settings = nn_settings
        self.max_mutations_per_clone = max_mutations_per_clone
        self.progression = progression
        self.trained_model_path = trained_model_path
        self.population_path = population_path
        self.member_pad = member_pad
        self.spam_pad = spam_pad
        self.progression_pad = progression_pad
        self.pop_pad = pop_pad
        self.status_pad = status_pad
        self.conv_activation_functions = conv_activation_functions
        self.dense_activation_functions = dense_activation_functions
        self.final_activation_functions = final_activation_functions
        self.optimizers = optimizers
        self.batch_size = batch_size

        # TODO: hand-over
        self.augment_params = {
            "random_brightness_delta": 0.25 * 255.0,
            "random_brightness_seed": 42
        }

        self.tfr_train_ds = tf.data.TFRecordDataset(tfr_train)
        self.tfr_test_ds = tf.data.TFRecordDataset(tfr_test)
        self.tfr_verify_ds = tf.data.TFRecordDataset(tfr_verify)
        self.tfr_train = nn_utils.get_dataset(self.tfr_train_ds,
                                              self.batch_size,
                                              self.nn_settings["type"],
                                              self.nn_settings["input_size_x"] * self.nn_settings["input_size_y"],
                                              self.augment_params)
        self.tfr_test =  nn_utils.get_dataset(self.tfr_test_ds,
                                              self.batch_size,
                                              self.nn_settings["type"],
                                              self.nn_settings["input_size_x"] * self.nn_settings["input_size_y"],
                                              self.augment_params)
        self.tfr_verify = nn_utils.get_dataset(self.tfr_verify_ds,
                                               len(list(self.tfr_verify_ds.as_numpy_iterator())),
                                               "verify",
                                               self.nn_settings["input_size_x"] * self.nn_settings["input_size_y"],
                                               {})
        self.predicter = predicter
        self.predicter_error_threshold = predicter_error_threshold
        self.evolver_run_uuid = str(uuid.uuid4())
        self.temp_model_file = f"tmp_model_{self.evolver_run_uuid}.hdf5"
        self.evolver_tfkeras_fail_log = f"evolver_tfkeras_fail_log_{self.evolver_run_uuid}.jsonl"
        self.evolver_success_log = f"evolver_success_log_{self.evolver_run_uuid}.jsonl"
        self.evolver_compiledNN_fail_log = f"evolver_compiledNN_fail_log_{self.evolver_run_uuid}.jsonl"
        self.evolver_stats_file = f"evolver_stats_{self.evolver_run_uuid}.csv"
        self.tfr_verify_cnn_ds = []
        for sample in self.tfr_verify.as_numpy_iterator():
            self.tfr_verify_cnn_ds.append(array(sample))

        def flatten(the_lists): #TODO: make better
            result = []
            for _list in the_lists:
                for val in _list:
                    result.append(val)
            return result

        cnn_samples = [flatten(flatten(sample)) for sample in self.tfr_verify_cnn_ds]
        cnn_samples = [[float(v) for v in sample] for sample in cnn_samples]
        to_be_joined = []
        for sample in cnn_samples:
            to_be_joined.append(str(sample))
        self.cnn_sample_string = "\n".join(to_be_joined)
        self.input_shape = (self.nn_settings["input_size_x"],
                            self.nn_settings["input_size_y"],
                            self.nn_settings["input_channels"])
        self.failed_counter = 0
        self.save_failed_max = 100

    class PrintCallback(tf.keras.callbacks.Callback):
        def  __init__(self, pad, ep, eps):
            self.pad = pad
            self.ep = ep
            self.eps = eps
        def on_epoch_begin(self, epoch, logs=None):
            self.pad.print("Epoch: " + str(self.ep) + " of " + str(self.eps) + "\n")

    def flattened_n(self, genes):
        size = self.nn_settings["input_size_x"]
        for (t, n, k, a, m, bn, dr, s) in genes["conv_layers"]:
            size /= s
            if m != 0:
                size /= abs(m)
        return (int(size*size*genes["conv_layers"][-1][1]))

    def add_member(self, new_member: Individual, save_files=False):
        load_trained_models = False
        load_trained_convolutions = False
        if os.path.isfile(str(self.trained_model_path / (new_member.name + "_" + new_member.name + "_" + str(self.flattened_n(new_member.genes)) + "_" + self.nn_settings["type"] + ".hdf5"))):
            load_trained_models = True

        model = None

        if load_trained_models:
            if self.nn_settings["metric"] in ["f05", "f1", "f2"]:
                model = tf.keras.models.load_model(str(self.trained_model_path / (new_member.name + "_" + self.nn_settings["type"] + ".hdf5")),
                                                   custom_objects={"f05": nn_utils.f05, "f1": nn_utils.f1, "f2": nn_utils.f2})
            else:
                model = tf.keras.models.load_model(str(self.trained_model_path / (new_member.name + "_" + self.nn_settings["type"] + ".hdf5")))
        else:
            full_layer_list = nn_utils.conv_layer_list(new_member.genes, self.input_shape)
            full_layer_list.extend(nn_utils.dense_layer_list(new_member.genes, self.flattened_n(new_member.genes), self.nn_settings["final_layer_neurons"]))
            model = tf.keras.models.Sequential(full_layer_list)
        try:
            opt = tf.keras.optimizers.get(new_member.genes["optimizer"])
            opt.learning_rate = new_member.genes["initial_learning_rate"]
            if self.nn_settings["metric"] in ["f05", "f1", "f2"]:
                model.compile(
                    optimizer=opt,
                    loss=self.nn_settings["loss"],
                    metrics=[nn_utils.f05, nn_utils.f1, nn_utils.f2])
            else:
                model.compile(
                    optimizer=opt,
                    loss=self.nn_settings["loss"],
                    metrics=[self.nn_settings["metric"]])
        except Exception as e:
            print(e, "Killed at model.compile")
            print(new_member.genes)
            nn_utils.add_to_log("tfkeras compile failed",
                                        {"type": self.nn_settings["type"],
                                         "genes": new_member.genes},
                                        self.evolver_tfkeras_fail_log)
            return
        new_member.cost = nn_utils.cost(new_member.genes, self.nn_settings["final_layer_neurons"], self.nn_settings["cost_factor"])
        print("cost : ", new_member.cost)
        print(new_member.genes)
        model.summary()
        self.member_pad.print(new_member, self.nn_settings["type"], self.nn_settings["final_layer_neurons"])
        self.spam_pad.print("")
        if not load_trained_models:
            try:
                current_lr = new_member.genes["initial_learning_rate"]
                for ep in range(new_member.genes["trainingEpochs"]):
                    printCallback = self.PrintCallback(self.spam_pad, ep+1, new_member.genes["trainingEpochs"])
                    model.fit(
                                        self.tfr_train,
                                        #steps_per_epoch = self.nn_settings["batches_per_training_epoch"],
                                        epochs=1,
                                        callbacks=[printCallback],
                                        verbose = 1)
                    current_lr *= new_member.genes["learning_rate_factor_per_epoch"]
                    model.optimizer.lr.assign(current_lr)
            except Exception as e:
                print(e, "Killed at model.fit")
                print(new_member.genes)
                nn_utils.add_to_log("tfkeras fit failed",
                                          {"type": self.nn_settings["type"],
                                           "genes": new_member.genes},
                                          self.evolver_tfkeras_fail_log)
                return
        self.spam_pad.print("")
        result = model.evaluate(self.tfr_test, verbose = 0)#steps = self.nn_settings["batches_for_scoring_test"], verbose = 0)
        if self.nn_settings["metric"] in ["f05", "accuracy"]:
            new_member.score = result[1]
        elif self.nn_settings["metric"] == "f1":
            new_member.score = result[2]
        elif self.nn_settings["metric"] == "f2":
            new_member.score = result[3]
        tf.keras.models.save_model(model,
                                   str(self.trained_model_path / self.temp_model_file),
                                   save_format='h5')

        new_member.fitness = new_member.score - new_member.cost
        append = True
        distance = nn_utils.compiledNN_average_distance(model,
                                                      str(self.trained_model_path / self.temp_model_file),
                                                      self.tfr_verify,
                                                      self.cnn_sample_string,
                                                      self.predicter)
        (self.trained_model_path / self.temp_model_file).unlink()
        compiledNN_success = False
        if distance < self.predicter_error_threshold:
            compiledNN_success = True
        self.status_pad.print(str(compiledNN_success) + " (" + "%.4f" % distance  + ") " +
                                        str(new_member.score)[:5].rjust(5," ") + " " + str(new_member.cost)[:5].rjust(5," "))
        if (new_member.fitness > self.best_fitness.fitness) and compiledNN_success:
            self.best_fitness = new_member
            self.progression.append((self.generation,
                                     self.best_fitness.score,
                                     self.best_fitness.fitness,
                                     self.best_fitness.cost,
                                     self.best_fitness.name,
                                     len(self.members)+1,
                                     dict(new_member.genes)))
            self.progression_pad.print(self.progression)
            append = False
            if not load_trained_models:
                save_files = True
        if (new_member.score > self.best_score.score) and compiledNN_success:
            self.best_score = new_member
            if append:
                self.progression.append((self.generation,
                                         self.best_score.score,
                                         self.best_score.fitness,
                                         self.best_score.cost,
                                         self.best_score.name,
                                         len(self.members)+1,
                                         dict(new_member.genes)))
                self.progression_pad.print(self.progression)
                append = False
            if not load_trained_models:
                save_files = True
        if save_files and not load_trained_models:
            with open(str(self.population_path / (new_member.name + '.json')), 'w') as outfile:
                json.dump(dict(new_member.genes), outfile)
            tf.keras.models.save_model(model,
                                       str(self.trained_model_path / (new_member.name + "_" + self.nn_settings["type"] + ".hdf5")),
                                       save_format='h5')
        if compiledNN_success:
            self.members.append(new_member)
            nn_utils.add_to_log("add to population",
                                      {"name": new_member.name,
                                       "type": self.nn_settings["type"],
                                       "score": float(new_member.score),
                                       "fitness:": float(new_member.fitness),
                                       "cost": float(new_member.cost),
                                       "tf_cnn_distance": distance,
                                       "genes": new_member.genes},
                                      self.evolver_success_log)
            nn_utils.add_to_csv([new_member.name,
                                 self.nn_settings["metric"],
                                 float(new_member.cost)] + result + model.metrics_names,
                                 self.evolver_stats_file)
        else:
            self.failed_counter += 1
            nn_utils.add_to_log("CompiledNN Fail",
                                      {"name": new_member.name,
                                       "type": self.nn_settings["type"],
                                       "score": float(new_member.score),
                                       "fitness:": float(new_member.fitness),
                                       "cost": float(new_member.cost),
                                       "tf_cnn_distance": distance,
                                       "genes": new_member.genes},
                                      self.evolver_compiledNN_fail_log)
            if self.failed_counter < self.save_failed_max:
                tf.keras.models.save_model(model,
                                           str(self.trained_model_path / "failed" / (new_member.name + "_" + self.nn_settings["type"] + ".hdf5")),
                                           save_format='h5')
        tf.keras.backend.clear_session()

    def add_random_member(self):
        thisCLayers = []
        thisDLayers = []
        hiddenLayers = random.randint(1, 4)
        size = self.nn_settings["input_size_x"]
        for i in range(hiddenLayers):
            layer_type = random.choice(["SeparableConv2D", "Conv2D"])
            kernels = random.choice([1, 2, 4, 8, 16, 32, 64, 128])
            kernelsize = random.choice([3, 5])
            activation_function = random.choice(self.conv_activation_functions)
            maxpool = random.choice([2, -2, 0])
            if maxpool != 0:
                size /= abs(maxpool)
            batchnorm = random.choice([True, False])
            dropout = random.random()/2.0
            stride = 1
            if size > 2:
                stride = random.choice([1, 2])
            size /= stride
            thisCLayers.append((layer_type, kernels, kernelsize, activation_function, maxpool, batchnorm, dropout, stride))
            if size == 2:
                break
        hiddenLayers = random.randint(0, 3)
        for i in range(hiddenLayers):
            neurons = random.randint(2, 128)
            activation_function = random.choice(self.dense_activation_functions)
            batchnorm = random.choice([True, False])
            dropout = random.random()/2.0
            thisDLayers.append((neurons,activation_function, batchnorm, dropout))
        randomName = ""
        consonant = "bcdfghjklmnpqrstvwxz"
        vowel = "aeiouy"
        atConsonant = random.choice([True,False])
        for i in range(random.randint(3,9)):
            if atConsonant:
                randomName += random.choice(vowel)
            else:
                randomName += random.choice(consonant)
            atConsonant = not(atConsonant)
        self.add_member(Individual(name=randomName+"-"+str(self.generation),
                                  genes={"trainingEpochs": random.randint(1,4),
                                         "conv_layers": thisCLayers,
                                         "dense_layers": thisDLayers,
                                         "final_layer": (random.choice(self.final_activation_functions), random.choice([True,False])),
                                         "optimizer": random.choice(self.optimizers),
                                         "initial_learning_rate": random.randint(1,100)  * 0.001,
                                         "learning_rate_factor_per_epoch": random.uniform(0.1, 0.8)}))

    def remove_worst(self):
        worst_fitness_value = 10.0
        worst_fitness_index = -1
        best_score = 0
        best_index = -1
        for i in range(len(self.members)):
            if self.members[i].score > best_score:
                best_score = self.members[i].score
                best_index = i
        for i in range(len(self.members)):
            if self.members[i].fitness < worst_fitness_value:
                if i != best_index:
                    worst_fitness_value = self.members[i].fitness
                    worst_fitness_index = i
        del self.members[worst_fitness_index]

    def clone_member(self, parent):
        ep = parent.genes["trainingEpochs"]
        opt = parent.genes["optimizer"]
        clay = list(parent.genes["conv_layers"])
        dlay = list(parent.genes["dense_layers"])
        (fact, fbn) = parent.genes["final_layer"]
        lr = parent.genes["initial_learning_rate"]
        lrf = parent.genes["learning_rate_factor_per_epoch"]
        for i in range(random.choice(range(self.max_mutations_per_clone))+1):
            mutationtype = random.randint(0,10)
            if mutationtype == 0:
                ep = random.randint(ep-2, ep+2)
                if ep < 1:
                    ep = 1
                if ep > 20:
                    ep = 20
            elif mutationtype < 4:
                layer_index = 0
                mut = 1
                if len(dlay) > 0:
                    layer_index = random.randint(0, len(dlay)-1)
                    mut = random.choice(range(3))
                if mut == 0: # remove layer
                    if len(dlay) > 1:
                        del dlay[layer_index]
                elif mut == 1: # add layer
                    if len(dlay) > 4:
                        del dlay[layer_index]
                    else:
                        layer_index = random.randint(0, len(dlay))
                        dlay.insert(layer_index, (random.randint(2,64), random.choice(self.dense_activation_functions), random.choice([True,False]), random.random()/2))
                elif mut == 2: #change layer
                    dlay[layer_index] = (random.randint(math.floor(2+math.floor(dlay[layer_index][0] / 2)), 2*(dlay[layer_index][0])),
                                      random.choice(self.dense_activation_functions),
                                      random.choice([True,False]),
                                      random.random()/2.0)
            elif mutationtype < 6: # change conv layer
                layer_index = random.randint(0, len(clay)-1)
                size = self.nn_settings["input_size_x"]
                for (t, n, k, a, m, bn, dr, s) in clay[:layer_index]:
                    size /= s
                    if m != 0:
                        size /= abs(m)
                mut = random.choice(range(8))
                (clay_type, clay_kernels, clay_size, clay_act, clay_m, clay_bn, clay_dr, clay_s)  = clay[layer_index]
                if mut == 0:
                    clay[layer_index] = (random.choice(["SeparableConv2D", "Conv2D"]), clay_kernels, clay_size, clay_act, clay_m, clay_bn, clay_dr, clay_s)
                elif mut == 1:
                    clay[layer_index] = (clay_type, random.choice([4, 8, 16, 32, 64, 128]), clay_size, clay_act, clay_m, clay_bn, clay_dr, clay_s)
                elif mut == 2:
                    clay[layer_index] = (clay_type, clay_kernels, random.choice([3, 5]), clay_act, clay_m, clay_bn, clay_dr, clay_s)
                elif mut == 3:
                    clay[layer_index] = (clay_type, clay_kernels, clay_size, random.choice(self.conv_activation_functions), clay_m, clay_bn, clay_dr, clay_s)
                elif mut == 4:
                    if size > 2:
                        clay[layer_index] = (clay_type, clay_kernels, clay_size, clay_act, random.choice([2, -2, 0]), clay_bn, clay_dr, clay_s)
                    else:
                        clay[layer_index] = (clay_type, clay_kernels, clay_size, clay_act, 0, clay_bn, clay_dr, clay_s)
                elif mut == 5:
                    clay[layer_index] = (clay_type, clay_kernels, clay_size, clay_act, clay_m, random.choice([True,False]), clay_dr, clay_s)
                elif mut == 6:
                    clay[layer_index] = (clay_type, clay_kernels, clay_size, clay_act, clay_m, clay_bn, min(clay_dr*(0.5+random.random()) ,0.5), clay_s)
                elif mut == 7:
                    if size > 2:
                        clay[layer_index] = (clay_type, clay_kernels, clay_size, clay_act, clay_m, clay_bn, clay_dr, random.choice([1,2]))
                    else:
                        clay[layer_index] = (clay_type, clay_kernels, clay_size, clay_act, clay_m, clay_bn, clay_dr, 1)
                size = self.nn_settings["input_size_x"]
                last_valid_index = -1
                for (t, n, k, a, m, bn, dr, s) in clay:
                    size /= s
                    if m != 0:
                        size /= abs(m)
                    if size >= 2:
                        last_valid_index += 1
                    else:
                        break
                clay = clay[:last_valid_index+1]
            elif mutationtype == 6: # add/remove conv layer
                add = random.choice([True,False])
                final_size = self.nn_settings["input_size_x"]
                for (t, n, k, a, m, bn, dr, s) in clay:
                    final_size /= s
                    if m != 0:
                        final_size /= abs(m)
                if final_size <= 2:
                    add = False
                if len(clay) == 1:
                    add = True
                if add:
                    layer_index = random.randint(0, len(clay))
                    layer_type = random.choice(["SeparableConv2D", "Conv2D"])
                    kernels = random.choice([1, 2, 4, 8, 16, 32, 64, 128])
                    kernelsize = random.choice([3, 5])
                    activation_function = random.choice(self.conv_activation_functions)
                    maxpool = random.choice([2, -2, 0])
                    if maxpool != 0:
                        final_size /= abs(maxpool)
                    batchnorm = random.choice([True, False])
                    dropout = random.random()/2.0
                    stride = 1
                    if final_size > 2:
                        stride = random.choice([1, 2])
                    clay.insert(layer_index, (layer_type, kernels, kernelsize, activation_function, maxpool, batchnorm, dropout, stride))
                else:
                    layer_index = random.randint(0, len(clay)-1)
                    del clay[layer_index]
            elif mutationtype == 7:
                lr *= random.uniform(lr/2, lr*2)
            elif mutationtype == 8:
                lrf *= random.uniform(lrf/2, lrf*2)
            elif mutationtype == 9:
                fact = random.choice(self.final_activation_functions)
                fbn = random.choice([True, False])
            elif mutationtype == 10:
                opt = random.choice(self.optimizers)
        self.add_member(Individual(name=parent.name.split('-', 1)[0][1:]+random.choice("02468")+"-"+str(self.generation),
                                  genes={"trainingEpochs": ep,
                                         "conv_layers": clay,
                                         "dense_layers": dlay,
                                         "final_layer": (fact, fbn),
                                         "optimizer": opt,
                                         "initial_learning_rate": lr, # random.randint(1,100)  * 0.001,
                                         "learning_rate_factor_per_epoch": lrf}))

    def pair_members(self, parent_a, parent_b):
        trainingEpochs = parent_a.genes["trainingEpochs"]
        clay = list(parent_a.genes["conv_layers"])
        dlay = list(parent_a.genes["dense_layers"])
        (fact, fbn) = parent_a.genes["final_layer"]
        opt = parent_a.genes["optimizer"]
        lr = parent_a.genes["initial_learning_rate"]
        lrf = parent_a.genes["learning_rate_factor_per_epoch"]
        if random.choice([True,False]):
            trainingEpochs = parent_b.genes["trainingEpochs"]
        if random.choice([True,False]):
            clay = list(parent_b.genes["conv_layers"])
        if random.choice([True,False]):
            dlay = list(parent_b.genes["dense_layers"])
        if random.choice([True,False]):
            opt = parent_b.genes["optimizer"]
        if random.choice([True,False]):
            (fact, fbn) = parent_b.genes["final_layer"]
        if random.choice([True,False]):
            lr = parent_b.genes["initial_learning_rate"]
        if random.choice([True,False]):
            lrf = parent_b.genes["learning_rate_factor_per_epoch"]
        parent_a_name = parent_a.name.split('-', 1)[0]
        parent_b_name = parent_b.name.split('-', 1)[0]
        newName = (parent_a_name + parent_b_name)[:12]
        if random.choice([True,False]):
            newName = (parent_b_name + parent_a_name)[:12]
        self.clone_member(Individual(name=newName+random.choice("13579")+"-"+str(self.generation),
                                  genes={"trainingEpochs": trainingEpochs,
                                         "conv_layers": clay,
                                         "dense_layers": dlay,
                                         "final_layer": (fact, fbn),
                                         "optimizer": opt,
                                         "initial_learning_rate": lr,
                                         "learning_rate_factor_per_epoch": lrf}))

    def fill_with_random(self):
        while len(self.members) < self.min_size:
            self.pop_pad.print(self)
            self.status_pad.print("+ random individual")
            self.add_random_member()
        self.pop_pad.print(self)

    def cull(self):
        while len(self.members) > self.max_size:
            self.remove_worst()
            self.pop_pad.print(self)

        while len(self.members) > self.min_size and random.choice([True, False]):
            self.remove_worst()
            self.pop_pad.print(self)

    def new_clone(self):
        parent = random.choice(self.members)
        self.status_pad.print("+ clone of " + parent.name)
        self.clone_member(parent)
        self.pop_pad.print(self)

    def new_random(self):
        self.status_pad.print("+ random individual")
        self.add_random_member()
        self.pop_pad.print(self)

    def shorten_name(self, name):
        if len(name) < 12:
            return name
        else:
            return (name[:5]+"..."+name[-5:])

    def new_offspring(self):
        parent_a = random.choice(self.members)
        parent_b = random.choice(self.members)
        self.status_pad.print("+ child of " +
                              self.shorten_name(parent_a.name) +
                              " and "+
                              self.shorten_name(parent_b.name))
        self.pair_members(parent_a, parent_b)
        self.pop_pad.print(self)

    def evolve(self):
        self.progression_pad.print(self.progression)
        if len(self.members) < self.min_size:
            self.status_pad.print("Create randoms until pop-minsize(" + str(self.min_size) + ") is reached")
            self.fill_with_random()
        self.generation += 1
        if random.choice([True, False]):
            if random.choice([True, False]):
                self.new_clone()
            else:
                self.new_random()
        else:
            self.new_offspring()
        if len(self.members) > self.max_size:
            self.status_pad.print("Culling Population")
            self.cull()
        self.pop_pad.print(self)
