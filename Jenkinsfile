pipeline {
  agent {
    label 'raven-windows'
  }
  stages {
    stage('build') {
      steps {
        script {
          sh 'cd lib && cargo build && cargo build --release && cargo test'
          sh 'cd app && cargo build && cargo build --release && cargo test'
          bat 'cd python & cargo build & cargo build --release & build_wheel.bat & test.bat'
        }
      }
    }
  }
  post {
    always {
      archiveArtifacts artifacts: 'app/target/release/ese_parser.exe,python/target/wheels/ese_parser-0.1.0-cp36-none-win_amd64.whl', onlyIfSuccessful: true
    }
  }
}
