import json

with open('etc/motions/standUpBack_dortmund.motion2') as f:
    motion_file = json.load(f)

joint_mapping = { joint_index: index for index, joint_index in enumerate(motion_file['header']['joints']) }

time_sum = sum(frame['time'] for frame in motion_file['position'])
header_time = motion_file['header']['time']

initial_positions = motion_file['position'][0]['parameters']
mapped_initial_positions = [ initial_positions[joint_mapping[index]] for index in range(len(initial_positions)) ]

def convert_positions_to_struct(positions):
    return {
        'head': {
            'yaw': positions[0],
            'pitch': positions[1],
        },
        'left_arm': {
            'shoulder_pitch': positions[2],
            'shoulder_roll': positions[3],
            'elbow_yaw': positions[4],
            'elbow_roll': positions[5],
            'wrist_yaw': positions[6],
            'hand': positions[7],
        },
        'right_arm': {
            'shoulder_pitch': positions[20],
            'shoulder_roll': positions[21],
            'elbow_yaw': positions[22],
            'elbow_roll': positions[23],
            'wrist_yaw': positions[24],
            'hand': positions[25],
        },
        'left_leg': {
            'hip_yaw_pitch': positions[8],
            'hip_roll': positions[9],
            'hip_pitch': positions[10],
            'knee_pitch': positions[11],
            'ankle_pitch': positions[12],
            'ankle_roll': positions[13],
        },
        'right_leg': {
            'hip_yaw_pitch': positions[14],
            'hip_roll': positions[15],
            'hip_pitch': positions[16],
            'knee_pitch': positions[17],
            'ankle_pitch': positions[18],
            'ankle_roll': positions[19],
        },
    }

output = {
    'initial_positions': convert_positions_to_struct(mapped_initial_positions),
    'frames': [],
}

for frame in motion_file['position'][1:]:
    time = frame['time']
    positions = frame['parameters']
    # print(positions)
    mapped_positions = [ positions[joint_mapping[index]] for index in range(len(positions)) ]
    # print(mapped_positions)
    # print(time / time_sum * header_time / 1000)
    # print()
    output['frames'].append({
        'duration': time / time_sum * header_time / 1000,
        'positions': convert_positions_to_struct(mapped_positions),
    })

# print(time_sum)
# print(header_time)

# print(output)

print(json.dumps(output, indent=2))
