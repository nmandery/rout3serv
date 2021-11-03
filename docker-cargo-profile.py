import toml

contents = toml.loads(open("Cargo.toml").read())
if not 'profile' in contents:
    contents['profile'] = {}
if not 'release' in contents['profile']:
    contents['profile']['release'] = {}

# size and performance optimized settings
contents['profile']['release']['lto'] = 'thin'
contents['profile']['release']['debug'] = 1
contents['profile']['release']['opt-level'] = 3
contents['profile']['release']['codegen-units'] = 1
print(contents)
open("Cargo.toml", 'w').write(toml.dumps(contents))
