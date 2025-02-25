#!/usr/bin/env groovy
@Grab(group = 'com.github.zafarkhaja', module = 'java-semver', version = '0.9.0')
import com.github.zafarkhaja.semver.Version

def executeOnShell(String command, Closure closure = null) {
  executeOnShell(command, new File(System.properties.'user.dir'), closure)
}

def executeOnShell(String command, File workingDir, Closure closure = null) {
  println command
  def processBuilder = new ProcessBuilder(['sh', '-c', command])
    .directory(workingDir)
  if (closure) {
    processBuilder.redirectErrorStream(true)
  } else {
    processBuilder.inheritIO()
  }
  def process = processBuilder.start()
  if (closure) {
    process.inputStream.eachLine closure
  }
  process.waitFor()
  if (process.exitValue() > 0) {
    System.exit(process.exitValue())
  }
}

void ask(String prompt, String defaultValue = 'Y', Closure cl) {
  def promptValue = System.console().readLine(prompt + ' ').trim()
  if (promptValue.empty) {
    promptValue = defaultValue
  }
  if (promptValue.toUpperCase() == 'Y') {
    cl.call()
  }
}

executeOnShell 'git pull'

ask('Execute Build?: [Y]') {
  executeOnShell 'cargo clean'
  executeOnShell 'mkdir -p ../target/artifacts'
  executeOnShell 'cargo build'
  executeOnShell 'cargo test'
}

ask('Execute CMake Build?: [Y]') {
  executeOnShell 'mkdir -p build'
  executeOnShell 'cd build && cmake ..'
  executeOnShell 'cd build && cmake --build .'
}

def projectProps = new File('Cargo.toml').text
def versionMatch = projectProps =~ /(?m)version\s*=\s*"(.*)"/
def version = versionMatch[0][1]

def prevTag = 'git describe --abbrev=0  --tags --match=libpact_ffi-*'.execute().text.trim()
def changelog = []
executeOnShell("git log --pretty='* %h - %s (%an, %ad)' ${prevTag}..HEAD .".toString()) {
  println it
  changelog << it
}

def releaseDesc = System.console().readLine('Describe this release: [Bugfix Release]').trim()
if (releaseDesc.empty) {
  releaseDesc = 'Bugfix Release'
}

def releaseVer = System.console().readLine("What is the version for this release?: [$version]").trim()
if (releaseVer.empty) {
  releaseVer = version
}

ask('Update Changelog?: [Y]') {
  def changeLogFile = new File('CHANGELOG.md')
  def changeLogFileLines = changeLogFile.readLines()
  changeLogFile.withPrintWriter() { p ->
    p.println(changeLogFileLines[0])

    p.println()
    p.println("# $releaseVer - $releaseDesc")
    p.println()
    changelog.each {
      p.println(it)
    }

    changeLogFileLines[1..-1].each {
      p.println(it)
    }
  }

  executeOnShell("git add CHANGELOG.md")
  executeOnShell("git commit -m 'update changelog for release $releaseVer'")
  executeOnShell("git status")
  executeOnShell("git diff HEAD^..HEAD")
}

ask('Tag and Push commits?: [Y]') {
  executeOnShell 'git push'
  executeOnShell("git tag libpact_ffi-v${releaseVer}")
  executeOnShell 'git push --tags'
}

ask('Publish library to crates.io?: [Y]') {
  executeOnShell 'rm -rf build'
  executeOnShell 'cargo package'
  executeOnShell 'cargo publish'
}

ask('Publish Conan packages?: [Y]') {
  executeOnShell "cd conan/lib && conan create . pact/beta && CONAN_REVISIONS_ENABLED=1 conan upload pact_ffi/${releaseVer}@pact/beta -r=pact-foundation"
  executeOnShell "cd conan/dll && conan create . pact/beta && CONAN_REVISIONS_ENABLED=1 conan upload pact_ffi_dll/${releaseVer}@pact/beta -r=pact-foundation"
}

def nextVer = Version.valueOf(releaseVer).incrementPatchVersion()
ask("Bump version to $nextVer?: [Y]") {
  executeOnShell "sed -i -e 's/version = \"${releaseVer}\"/version = \"${nextVer}\"/' Cargo.toml"
  executeOnShell "sed -i -e 's/version = \"${releaseVer}\"/version = \"${nextVer}\"/' conan/lib/conanfile.py"
  executeOnShell "sed -i -e 's/version = \"${releaseVer}\"/version = \"${nextVer}\"/' conan/dll/conanfile.py"
  executeOnShell("cargo update")
  executeOnShell("git add Cargo.toml README.md conan/lib/conanfile.py conan/dll/conanfile.py")
  executeOnShell("git add ../Cargo.lock")
  executeOnShell("git diff --cached")
  ask("Commit and push this change?: [Y]") {
    executeOnShell("git commit -m 'bump version to $nextVer'")
    executeOnShell("git push")
  }
}
