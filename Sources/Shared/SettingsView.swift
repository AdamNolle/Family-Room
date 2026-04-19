import SwiftUI

struct SettingsView: View {
    @ObservedObject var viewModel: AppViewModel
    
    var body: some View {
        ZStack {
            AdaptiveFabricBackground(type: viewModel.selectedFabric)
            
            VStack(alignment: .leading, spacing: 30) {
                Text("Settings")
                    .font(.system(size: 34, weight: .black, design: .serif))
                    .foregroundStyle(viewModel.selectedFabric == .wood ? .white : FurnitureDesign.walnutWood)
                    .padding(.horizontal)
                    .padding(.top, 40)
                
                VStack(alignment: .leading, spacing: 15) {
                    Text("Interior Design")
                        .font(.headline)
                        .padding(.horizontal)
                    
                    VStack {
                        ForEach(AppViewModel.FabricType.allCases, id: \.self) { fabric in
                            Button {
                                withAnimation(.spring()) {
                                    viewModel.selectedFabric = fabric
                                }
                            } label: {
                                HStack {
                                    Text(fabric.rawValue)
                                    Spacer()
                                    if viewModel.selectedFabric == fabric {
                                        Image(systemName: "checkmark.circle.fill")
                                    }
                                }
                                .padding()
                                .background(viewModel.selectedFabric == fabric ? .white.opacity(0.2) : .clear)
                                .clipShape(RoundedRectangle(cornerRadius: 12))
                            }
                            .buttonStyle(.plain)
                        }
                    }
                    .padding()
                    .background(.ultraThinMaterial)
                    .clipShape(RoundedRectangle(cornerRadius: 20))
                    .padding(.horizontal)
                }
                
                Spacer()
            }
        }
    }
}
