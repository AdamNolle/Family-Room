import SwiftUI

struct HomeView: View {
    @ObservedObject var viewModel: AppViewModel
    
    var body: some View {
        NavigationStack {
            ZStack {
                AdaptiveFabricBackground(type: viewModel.selectedFabric)
                
                ScrollView {
                    VStack(alignment: .leading, spacing: 40) {
                        // Header
                        VStack(alignment: .leading, spacing: 4) {
                            Text("Home")
                                .font(.system(size: 34, weight: .black, design: .serif)) // Serif for more traditional feel
                                .foregroundStyle(FurnitureDesign.walnutWood)
                            Text("Your Family Memories")
                                .font(.subheadline)
                                .foregroundStyle(FurnitureDesign.sageWool)
                        }
                        .padding(.horizontal)
                        .padding(.top, 40)
                        
                        // Featured Album
                        VStack(alignment: .leading, spacing: 20) {
                            Text("Featured Album")
                                .font(.headline)
                                .foregroundStyle(FurnitureDesign.walnutWood)
                                .padding(.horizontal)
                            
                            FeaturedAlbumView()
                                .padding(.horizontal)
                        }
                        
                        // All Albums Grid
                        VStack(alignment: .leading, spacing: 20) {
                            Text("Recent Albums")
                                .font(.headline)
                                .foregroundStyle(FurnitureDesign.walnutWood)
                                .padding(.horizontal)
                            
                            LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 30) {
                                ForEach(viewModel.recentEvents) { event in
                                    NavigationLink(value: event) {
                                        AlbumStackThumbnail(event: event)
                                    }
                                    .buttonStyle(.plain)
                                }
                            }
                            .padding(.horizontal)
                        }
                    }
                    .padding(.bottom, 100)
                }
            }
            .navigationDestination(for: FamilyEvent.self) { event in
                EventDetailView(event: event)
                    .navigationBarBackButtonHidden()
            }
        }
    }
}

struct FeaturedAlbumView: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 15) {
            ZStack {
                RoundedRectangle(cornerRadius: 8)
                    .fill(FurnitureDesign.sageWool.opacity(0.8))
                    .frame(height: 250)
                
                Image(systemName: "photo.stack.fill")
                    .font(.system(size: 80))
                    .foregroundStyle(.white.opacity(0.5))
            }
            .albumStyle()
            
            VStack(alignment: .leading, spacing: 5) {
                Text("Summer at the Lake")
                    .font(.title2.bold().serif())
                Text("248 items • Curated by Dad")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
        }
    }
}

struct AlbumStackThumbnail: View {
    let event: FamilyEvent
    
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            ZStack {
                Rectangle()
                    .fill(FurnitureDesign.walnutWood.opacity(0.1))
                    .aspectRatio(1, contentMode: .fit)
                
                Image(systemName: "photo")
                    .foregroundStyle(.secondary)
            }
            .albumStyle()
            
            VStack(alignment: .leading, spacing: 2) {
                Text(event.name)
                    .font(.subheadline.bold())
                    .foregroundStyle(FurnitureDesign.walnutWood)
                Text(event.date)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
    }
}

extension Font {
    func serif() -> Font {
        return self // Simplified for now
    }
}
