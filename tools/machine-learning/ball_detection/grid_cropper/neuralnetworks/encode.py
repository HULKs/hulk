import binascii
import sys

with open(sys.argv[1], 'rb') as f:
    contents = f.read()

hex_string = binascii.hexlify(contents).decode('utf-8')

with open(sys.argv[3], 'w') as f:
    # produces a string literal with octal values
    print('static const char*', sys.argv[2], '= "', file=f, end='')
    for i in range(0, len(hex_string), 2):
        if i % 100 == 0 and i != 0:
            print('"\n"', file=f, end='')
        print('\\' + oct(int(hex_string[i:i+2], 16))[2:], file=f, end='')
    print('";\nstatic const size_t', f'{sys.argv[2].upper()}_SIZE', '=', len(hex_string) // 2, ';', file=f)
