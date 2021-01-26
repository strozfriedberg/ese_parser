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
        }
      }
    }
  }
  post {
    always {
      archiveArtifacts artifacts: 'app/target/release/ese_parser.exe', onlyIfSuccessful: true
    }
  }
}
